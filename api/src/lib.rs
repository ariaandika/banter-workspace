use std::sync::Arc;
use sqlx::{postgres::PgRow, prelude::*};
use http_core::{*, util::*};
use sqlx::PgPool;



pub async fn router(parts: &Parts, _: Body, state: Arc<PgPool>) -> Result {
    let path = normalize_path(parts.uri.path());

    match (&parts.method, path) {
        (GET, "/") => {
            let us = sqlx::query(sql::SELECT_USERS)
                .map(|e: PgRow|e.get::<String, _>("name"))
                .fetch_all(&*state).await.fatal()?;
            us.into_response()
        }
        _ => NOT_FOUND,
    }
}





