use std::{env::var, sync::{Arc, LazyLock}};
use http_body_util::BodyExt;
use hyper::{body::Body as _, StatusCode};
use serde::Serialize;
use sqlx::{postgres::PgRow, prelude::*};
use http_core::{*, util::*};
use sqlx::PgPool;
use types::Deserialize;

/// 64kb
const MAX_PAYLOAD: u64 = 1024 * 64;
const SESSION_KEY: &str = "access_token";
const JWT_SECRET: LazyLock<String> = LazyLock::new(||var("JWT_SECRET").expect("checked"));

pub async fn router(parts: &Parts, body: Body, state: Arc<PgPool>) -> Result {
    if &parts.method != GET && body.size_hint().upper().unwrap_or(u64::MAX) > MAX_PAYLOAD  {
        return Err(Error::Http(StatusCode::PAYLOAD_TOO_LARGE));
    }

    let path = normalize_path(parts.uri.path());

    match (&parts.method, path) {
        (GET, "/") => {
            let us = sqlx::query(sql::SELECT_USERS)
                .map(|e: PgRow|e.get::<String, _>("name"))
                .fetch_all(&*state).await.fatal()?;
            us.into_response()
        }
        (GET, "/auth") => session(&*JWT_SECRET, SESSION_KEY, &parts)?.into_response(),

        (POST, "/login") => {
            #[derive(Serialize, Deserialize)]
            struct Login {
                phone: String,
                password: String
            }
            // LATEST: return error based on accept header
            let login = serde_json::from_slice::<Login>(&body.collect().await.unwrap().to_bytes())
                .bad_request()?;

            auth::sign::sign(&*JWT_SECRET, &serde_json::to_string(&login).expect("deez"))
                .into_response()
        }

        _ => NOT_FOUND,
    }
}

