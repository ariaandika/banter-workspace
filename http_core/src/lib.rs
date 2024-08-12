use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, StatusCode};

pub type Request = hyper::Request<Incoming>;
pub type Response = hyper::Response<Full<Bytes>>;
pub type Result<T = Response, E = Error> = std::result::Result<T,E>;

pub const NOT_FOUND: Result = Err(Error::Http(StatusCode::NOT_FOUND));

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



