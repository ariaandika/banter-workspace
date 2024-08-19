use std::{env::var, sync::LazyLock};
use auth::{mock_verify, sign::sign, verify_passwd, Error as AuthError, Role::Sales, SalesData, Token};
use http_body_util::BodyExt as _;
use hyper::{body::Body as _, header::SET_COOKIE, StatusCode};
use serde::Serialize;
use serde_json::Value;
use sql::*;
use sqlx::{postgres::PgRow, prelude::*, PgConnection};
use http_core::*;
use sqlx::PgPool;
use tokio::task::spawn_blocking;
use types::{Deserialize, Destination, OrderId, Orders, Package, UserAnon, UserSid, Users};

/// 64kb
const MAX_PAYLOAD: u64 = 1024 * 64;
const JWT_SECRET: LazyLock<String> = LazyLock::new(||var("JWT_SECRET").expect("checked"));
const BASE: &str = "";

pub async fn handle(request: Request, state: PgPool) -> Response {
    let (parts, body) = request.into_parts();
    match router(&parts, body, &state).await {
        Ok(ok) => ok,
        Err(err) => Response::builder()
            .status(err.status())
            .json(json!{{ "error": err.error(), "message": err_msg(err) }})
            .expect(concat!("deez ",line!())),
    }
}

fn err_msg(err: Error) -> String {
    match err {
        Error::InternalError(msg) => {eprintln!("{msg}");"Internal Server Error".into()},
        e => e.message(),
    }
}

pub async fn router(parts: &Parts, body: Body, state: &PgPool) -> Result {
    if &parts.method != GET && body.size_hint().upper().unwrap_or(u64::MAX) > MAX_PAYLOAD  {
        return Err(Error::Http(StatusCode::PAYLOAD_TOO_LARGE));
    }

    let path = parts.normalize_path();

    if path == "/login" ||
        path == "/logout" ||
        path.starts_with("/auth") {
        return handle_auth(parts, body, state).await;
    }

    if path.starts_with("/orders") {
        return handle_orders(parts, body, state).await;
    }

    if path.starts_with("/sales") {
        return handle_sales(parts, body, state).await;
    }

    match (&parts.method, path) {
        (GET, "/") => {
            let us = sqlx::query(sql::SELECT_USERS)
                .map(|e: PgRow|e.get::<String, _>("name"))
                .fetch_all(state).await.fatal()?;
            us.into_response()
        }
        _ => NOT_FOUND,
    }
}

async fn handle_auth(parts: &Parts, body: Body, state: &PgPool) -> Result {
    const SECURE: &str = if cfg!(debug_assertions) { "" } else { "; Secure" };
    const LOGOUT_COOKIE: &str = if cfg!(debug_assertions) {
        "access_token=; Path=/; Expires=Fri, 1 Jan 2010 00:00:00 UTC; HttpOnly; SameSite=None"
    } else {
        "access_token=; Path=/; Expires=Fri, 1 Jan 2010 00:00:00 UTC; HttpOnly; SameSite=None; Secure"
    };

    let path = parts.normalize_path();

    if &parts.method == POST && path == "/login" {
        #[derive(Serialize, Deserialize)]
        struct Login {
            phone: String,
            password: String
        }

        let login = serde_json::from_slice::<Login>(&body.collect().await?.to_bytes()).bad_request()?;

        let Some(user) = sqlx::query_as::<_, Users>(sql::FIND_USERS_BY_PHONE)
            .bind(&login.phone).fetch_optional(state).await.fatal()? else
        {
            let _ = spawn_blocking(move ||mock_verify(&login.password)).await.fatal()?;
            return Err(Error::Auth(AuthError::InvalidCredential));
        };

        let hashed = user.password.clone();

        if spawn_blocking(move ||verify_passwd(&login.password, &hashed))
            .await.fatal()?.map_err(|e|Error::InternalError(e.to_string()))?.is_none()
        {
            return Err(Error::Auth(AuthError::InvalidCredential));
        };

        let token = Token::new(user, Value::Null);
        let token_str = sign(&*JWT_SECRET, &serde_json::to_string(&token).expect("deez"));
        let cookie = format!("access_token={token_str}; Path=/; Expires=Fri, 1 Jan 2010 00:00:00 UTC; HttpOnly; SameSite=None{SECURE}");

        return Response::builder().header(SET_COOKIE, cookie).json(token);
    }

    match (&parts.method, path) {
        (GET, "/logout") => Response::builder().header(SET_COOKIE, LOGOUT_COOKIE).empty(),
        (GET, "/auth") => parts.get_session()?.into_response(),
        _ => NOT_FOUND,
    }
}

async fn handle_orders(parts: &Parts, _: Body, state: &PgPool) -> Result {
    let path = parts.normalize_prefix("/orders");
    let (limit,page) = parts.parse_query();
    match (&parts.method, path) {
        (GET, BASE) => sqlx::query_as::<_, Orders>(SELECT_ORDERS)
            .bind(limit as i32).bind(page as i32).fetch_all(state).await.fatal()?
            .into_response(),
        (GET, "/tracings") => sqlx::query_as::<_, Orders>(SELECT_ORDERS_TRACINGS)
            .bind(limit as i32).bind(page as i32).fetch_all(state).await.fatal()?
            .into_response(),
        _ => NOT_FOUND,
    }
}

async fn handle_sales(parts: &Parts, body: Body, state: &PgPool) -> Result {
    let path = parts.normalize_prefix("/sales");

    let (_session, sales) = parts.get_session_role(Sales)?.split::<SalesData>()?;
    let (limit,page) = parts.parse_query();

    match (&parts.method, path) {
        (GET, BASE) => sqlx::query_as::<_, Orders>(SELECT_ORDER_STATUS_BY_WH_ID)
            .bind(&sales.wh_id).bind(limit as i32).bind(page as i32).fetch_all(state).await.fatal()?
            .into_response(),
        (POST, BASE) => create_order(parts, &body.json().await?, &state).await?.into_response(),
        _ => NOT_FOUND,
    }
}

#[derive(Deserialize)]
struct CreateOrder {
    sender: UserAnon,
    receiver: UserAnon,
    destination: Destination,
    packages: Vec<Package>
}

async fn create_order(_: &Parts, data: &CreateOrder, state: &PgPool) -> Result<OrderId> {
    let mut tx = state.begin().await.fatal()?;

    let sender_sid = snapshot_anon(&data.sender, &mut *tx).await?;
    let receiver_sid = snapshot_anon(&data.receiver, &mut *tx).await?;

    let res = sqlx::query_scalar(INSERT_ORDERS)
        .bind(&sender_sid).bind(&receiver_sid)
        .bind(&data.destination.json_str()?).bind(&data.packages.json_str()?)
        .fetch_one(&mut *tx).await.fatal()?;

    tx.commit().await.fatal()?;

    Ok(res)
}

async fn snapshot_anon(anon: &UserAnon, state: &mut PgConnection) -> Result<UserSid> {
    match &anon.user_id {
        Some(id) => match sqlx::query_scalar(ID_USERS_SN).fetch_optional(state).await.fatal()? {
            Some(row) => Ok(row),
            None => Err(Error::Logic(http_core::LogicError::UserIdNotFound(id.0))),
        },
        None => sqlx::query_scalar(CREATE_USERS_SN)
            .bind(anon.json_str()?).fetch_one(state).await.fatal(),
    }
}

