pub use hyper::{body::Incoming as Body, http::request::Parts};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, StatusCode};

pub type Request = hyper::Request<Body>;
pub type Response = hyper::Response<Full<Bytes>>;
pub type Result<T = Response, E = Error> = std::result::Result<T,E>;

pub const NOT_FOUND: Result = Err(Error::Http(StatusCode::NOT_FOUND));
pub const GET: &Method = &Method::GET;
pub const POST: &Method = &Method::POST;

pub enum Error {
    Http(StatusCode),
}

impl Error {
    pub fn into_response(self) -> Response {
        match self {
            Error::Http(status) => hyper::Response::builder()
                .status(status)
                .body(Full::new(Bytes::new()))
                .expect("infallible"),
        }
    }
}

pub trait IntoResponse {
    fn into_response() -> Result;
}

pub mod util {
    pub fn normalize_path<'r>(path: &'r str) -> &'r str {
        match path {
            "/" => path,
            e if e.ends_with("/") => &e[..e.len()-1],
            e => e
        }
    }
}

