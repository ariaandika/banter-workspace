use std::sync::Arc;
use http_body_util::BodyExt;
use serde::Serialize;
use sqlx::{postgres::PgRow, prelude::*};
use http_core::{*, util::*};
use sqlx::PgPool;
use types::Deserialize;

const SESSION_KEY: &str = "access_token";

const JWT_TOKEN: &str = "jwt";

pub async fn router(parts: &Parts, body: Body, state: Arc<PgPool>) -> Result {
    let path = normalize_path(parts.uri.path());

    match (&parts.method, path) {
        (GET, "/") => {
            let us = sqlx::query(sql::SELECT_USERS)
                .map(|e: PgRow|e.get::<String, _>("name"))
                .fetch_all(&*state).await.fatal()?;
            us.into_response()
        }
        (GET, "/auth") => session(JWT_TOKEN, SESSION_KEY, &parts)?.into_response(),

        (POST, "/login") => {
            #[derive(Serialize, Deserialize)]
            struct Login {
                phone: String,
                password: String
            }
            // LATEST: tracings, return error based on accept header
            let login = serde_json::from_slice::<Login>(&body.collect().await.unwrap().to_bytes())
                .bad_request()?;

            login.into_response()
        }

        _ => NOT_FOUND,
    }
}

