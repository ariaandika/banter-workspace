use std::{env::var, future::Future, pin::Pin, process};
use hyper::{server::conn::http1::Builder, service::Service};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, runtime::Builder as Tokio, spawn};
use http_core::{Request, Response, NOT_FOUND};

fn main()  {
    Tokio::new_multi_thread()
        .enable_all().build()
        .unwrap().block_on(server());
}

async fn server() {
    let tcp = {
        let addr = format!("{}:{}",
            var("HOST").as_deref().unwrap_or("127.0.0.1"),
            var("PORT").as_deref().unwrap_or("3000"));
        match TcpListener::bind(&addr).await {
            Ok(tcp) => {
                println!("Listening http://{addr}");
                tcp
            },
            Err(err) => {
                eprintln!("cannot bind '{addr}', {err}");
                process::exit(1);
            }
        }
    };
    loop {
        let Ok((io, _)) = tcp.accept().await else { continue };
        spawn(Builder::new().serve_connection(TokioIo::new(io), Server));
    }
}

pub struct Server;

impl Service<Request> for Server {
    type Response = Response;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = hyper::Result<Response>> + Send>>;
    fn call(&self, _req: Request) -> Self::Future { Box::pin(router()) }
}

async fn router() -> hyper::Result<Response> {
    match NOT_FOUND {
        Ok(ok) => Ok(ok),
        Err(err) => Ok(err.into_response())
    }
}

