use std::{env::var, future::Future, pin::Pin, process, str::FromStr, sync::Arc};
use hyper::{server::conn::http1::Builder, service::Service};
use hyper_util::rt::TokioIo;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::{net::TcpListener, runtime::Builder as Tokio, spawn};
use http_core::{Request, Response};
use tracing::{error, info, info_span, Instrument};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

const DEFAULT_TRACE: &str = if cfg!(debug_assertions) {
    "trace,sqlx_postgres=off"
} else {
    "info"
};

fn main()  {
    if let Err(err) = Tokio::new_multi_thread()
        .enable_all().build()
        .unwrap().block_on(server())
    {
        error!(target: "main", "{err}");
        process::exit(1);
    }
}

async fn server() -> Result<(), String> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_str(&format!("{DEFAULT_TRACE},{}",
            if let Ok(ok) = EnvFilter::try_from_default_env()
            { ok.to_string() } else { DEFAULT_TRACE.into() }
        )).unwrap())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _ = dotenvy::dotenv();

    if let Err(_) = var("JWT_SECRET") { Err("JWT_SECRET: env required".to_string())? }

    let tcp = {
        let addr = var("HOST").unwrap_or("127.0.0.1".into())
            + ":" + var("PORT").as_deref().unwrap_or("3000");
        match TcpListener::bind(&addr).await {
            Ok(tcp) => { info!(target: "main", "Listening http://{addr}"); tcp },
            Err(err) => Err(format!("cannot bind `{addr}`, {err}"))?
        }
    };

    let pg_pool = match var("DATABASE_URL") {
        Ok(db_url) => match PgPoolOptions::new().connect_lazy(&db_url) {
            Ok(ok) => ok,
            Err(err) => Err(format!("DATABASE_URL: {err}"))?,
        },
        Err(err) => Err(format!("DATABASE_URL: {err}"))?,
    };

    let state = Arc::new(pg_pool);

    loop {
        let Ok((io, _)) = tcp.accept().await else { continue };
        spawn(Builder::new().serve_connection(TokioIo::new(io), Server(Arc::clone(&state))));
    }
}

pub struct Server(Arc<PgPool>);

impl Service<Request> for Server {
    type Response = Response;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = hyper::Result<Response>> + Send>>;
    fn call(&self, req: Request) -> Self::Future {
        let span = info_span!("","{}{}",req.method(),req.uri().path());
        Box::pin(router(req, Arc::clone(&self.0)).instrument(span))
    }
}

async fn router(req: Request, state: Arc<PgPool>) -> hyper::Result<Response> {
    let (parts, body) = req.into_parts();
    match api::router(&parts, body, state).await {
        Ok(ok) => Ok(ok),
        Err(err) => Ok(err.into_response())
    }
}

