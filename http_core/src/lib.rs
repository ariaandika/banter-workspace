use std::fmt::{Debug, Display, Formatter as Fmt, Result as FmtRes};

pub use hyper::{body::Incoming as Body, http::request::Parts};
pub use serde_json::json;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{header::CONTENT_TYPE, http::response::Builder as ResponseBuilder, Method, StatusCode};
use serde::Serialize;

pub type Request = hyper::Request<Body>;
pub type Response<T = Full<Bytes>> = hyper::Response<T>;
pub type Result<T = Response, E = Error> = std::result::Result<T,E>;

pub const NOT_FOUND: Result = Err(Error::Http(StatusCode::NOT_FOUND));
pub const GET: &Method = &Method::GET;
pub const POST: &Method = &Method::POST;

pub enum Error {
    Http(StatusCode),
    InternalError(String),
}

impl Error {
    pub fn into_response(self) -> Response {
        let build = match &self {
            Error::Http(status) => Response::builder().status(status),
            Error::InternalError(_) => Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR)
        };

        build.empty().expect("Infallible")
    }

    pub fn write(&self, f: &mut Fmt<'_>) -> std::fmt::Result {
        match &self {
            Error::Http(status) => write!(f, "{}", status.canonical_reason().unwrap_or("HttpError")),
            Error::InternalError(msg) => write!(f, "{msg}"),
        }
    }
}

pub fn body<T>(body: T) -> Full<Bytes> where Bytes: From<T> {
    Full::new(Bytes::from(body))
}

pub trait Builder {
    fn empty(self) -> Result;
    fn json<T>(self, json: T) -> Result where T: Serialize;
    fn html<T>(self, html: T) -> Result where Bytes: From<T>;
}

impl Builder for ResponseBuilder {
    fn empty(self) -> Result { Ok(self.body(Default::default())?) }
    fn json<T>(self, json: T) -> Result where T: Serialize {
        Ok(self
            .header(CONTENT_TYPE, "application/json")
            .body(body(serde_json::to_vec(&json)?))?)
    }
    fn html<T>(self, html: T) -> Result where Bytes: From<T> {
        Ok(self
            .header(CONTENT_TYPE, "text/html")
            .body(body(html))?)
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Result;
}

impl<S> IntoResponse for S where S: Serialize {
    fn into_response(self) -> Result { Response::builder().json(self) }
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



impl std::error::Error for Error { }
impl Debug for Error { fn fmt(&self, f: &mut Fmt<'_>) -> FmtRes { self.write(f) } }
impl Display for Error { fn fmt(&self, f: &mut Fmt<'_>) -> FmtRes { self.write(f) } }

macro_rules! fatal_err { ($id: path) => {
    impl From<$id> for Error { fn from(value: $id) -> Self { Self::InternalError(value.to_string()) } }
}}

fatal_err!(hyper::Error);
fatal_err!(hyper::http::Error);
fatal_err!(serde_json::Error);

pub trait ErrorExt<T> where Self: Sized {
    fn fatal(self) -> Result<T>;
}

impl<T, E> ErrorExt<T> for std::result::Result<T, E> where E: std::error::Error {
    fn fatal(self) -> Result<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(Error::InternalError(err.to_string())),
        }
    }
}

