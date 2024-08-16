use std::{borrow::Cow, env::var, fmt::{Debug, Display, Formatter as Fmt, Result as FmtRes}, sync::LazyLock};
use auth::{Error as AuthError, Role, Token};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{header::{AUTHORIZATION, CONTENT_TYPE, COOKIE}, http::response::Builder as ResponseBuilder, Method, StatusCode};
use serde::Serialize;

pub use hyper::{body::Incoming as Body, http::request::Parts};
pub use serde_json::json;

pub type Request = hyper::Request<Body>;
pub type Response<T = Full<Bytes>> = hyper::Response<T>;
pub type Result<T = Response, E = Error> = std::result::Result<T,E>;
pub type Paginate = (i32,i32);

pub const NOT_FOUND: Result = Err(Error::Http(StatusCode::NOT_FOUND));
pub const UNAUTHORIZED: Result = Err(Error::Auth(AuthError::Unauthorized));
pub const GET: &Method = &Method::GET;
pub const POST: &Method = &Method::POST;

pub enum Error {
    Http(StatusCode),
    BadRequest(String),
    InternalError(String),
    Auth(AuthError),
}

impl Error {
    pub fn into_response(self) -> Response {
        if let Error::InternalError(ref message) = self {
            tracing::error!(target: "InternalError",message);
        }

        let build = Response::builder();
        let build = match &self {
            Error::Http(status) => build.status(status),
            Error::InternalError(_) => build.status(StatusCode::INTERNAL_SERVER_ERROR),
            Error::Auth(err) => build.status(auth_status_code(&err)),
            Error::BadRequest(_) => build.status(StatusCode::BAD_REQUEST),
        };

        // TODO: write body based on accept header
        build.empty().expect("Infallible")
    }

    pub fn write(&self, f: &mut Fmt<'_>) -> std::fmt::Result {
        match &self {
            Error::Http(status) => write!(f, "{}", status.canonical_reason().unwrap_or("HttpError")),
            Error::InternalError(msg) => write!(f, "{msg}"),
            Error::Auth(err) => write!(f, "{err}"),
            Error::BadRequest(msg) => write!(f, "{msg}"),
        }
    }
}

#[inline]
fn auth_status_code(auth: &AuthError) -> StatusCode {
    match auth {
        AuthError::Forbidden => StatusCode::FORBIDDEN,
        _ => StatusCode::UNAUTHORIZED
    }
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
            .body(Full::new(Bytes::from(serde_json::to_vec(&json)?)))?)
    }
    fn html<T>(self, html: T) -> Result where Bytes: From<T> {
        Ok(self
            .header(CONTENT_TYPE, "text/html")
            .body(Full::new(Bytes::from(html)))?)
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Result;
}

impl<S> IntoResponse for S where S: Serialize {
    fn into_response(self) -> Result { Response::builder().json(self) }
}

const SESSION_KEY: &str = "access_token";
const JWT_SECRET: LazyLock<String> = LazyLock::new(||var("JWT_SECRET").expect("unchecked jwt secret"));

pub trait PartsExt<'r> {
    fn normalize_path(&'r self) -> &'r str;
    fn normalize_prefix(&'r self, prefix: usize) -> &'r str;
    fn parse_query(&'r self) -> Paginate;
    fn get_cookie(&'r self, key: &str) -> Option<&'r str>;
    fn auth_header(&'r self) -> Option<&'r str>;
    fn get_session(&'r self) -> Result<Token>;
    fn get_session_role(&'r self, role: Role) -> Result<Token>;
}

impl<'r> PartsExt<'r> for Parts {
    fn normalize_path(&'r self) -> &'r str {
        match self.uri.path() {
            e @ "/" => e,
            e if e.is_empty() => "/",
            e if e.ends_with("/") => &e[..e.len()-1],
            e => e
        }
    }

    // panic if path prefix not checked
    fn normalize_prefix(&'r self, prefix: usize) -> &'r str {
        match self.uri.path() {
            e @ "/" => e,
            e if e.is_empty() => "/",
            e if e.ends_with("/") => &e[prefix..e.len()-1],
            e => &e[prefix..]
        }
    }

    fn parse_query(&'r self) -> Paginate {
        fn par((_, v): (Cow<str>,Cow<str>)) -> Option<i32> { v.parse().ok() }
        let Some(q) = self.uri.query() else { return (20,0); };
        let mut qs = form_urlencoded::parse(q.as_bytes());
        let limit = qs.find(|(k,_)|k=="limit").and_then(par).unwrap_or(20);
        let page = qs.find(|(k,_)|k=="page").and_then(par).unwrap_or(0);
        (limit, limit * page)
    }

    fn get_cookie(&'r self, key: &str) -> Option<&'r str> {
        self.headers.get(COOKIE)?
            .to_str().ok()?.split('&')
            .find(|e|e.starts_with(key))?
            .split_once('=').map(|e|e.1)
    }

    fn auth_header(&'r self) -> Option<&'r str> {
        self.headers.get(AUTHORIZATION)?
            .to_str().ok()?.split_once(" ").map(|e|e.1)
    }

    fn get_session(&self) -> Result<Token> {
        match Token::from_token_str(&*JWT_SECRET,
            if let Some(t) = self.get_cookie(SESSION_KEY) { t }
            else if let Some(t) = self.auth_header() { t }
            else { return Err(Error::Auth(AuthError::Unauthorized)); })
        {
            Ok(ok) => Ok(ok),
            Err(err) => Err(Error::Auth(err)),
        }
    }

    fn get_session_role(&'r self, role: Role) -> Result<Token> {
        let s = self.get_session()?;
        match s.role == role {
            true => Ok(s),
            false => Err(Error::Auth(AuthError::Forbidden)),
        }
    }
}

impl std::error::Error for Error { }
impl Debug for Error { fn fmt(&self, f: &mut Fmt<'_>) -> FmtRes { self.write(f) } }
impl Display for Error { fn fmt(&self, f: &mut Fmt<'_>) -> FmtRes { self.write(f) } }
impl From<AuthError> for Error { fn from(value: AuthError) -> Self { Self::Auth(value) } }

macro_rules! fatal_err { ($id: path) => {
    impl From<$id> for Error { fn from(value: $id) -> Self { Self::InternalError(value.to_string()) } }
}}

fatal_err!(hyper::Error);
fatal_err!(hyper::http::Error);
fatal_err!(serde_json::Error);

pub trait ErrorExt<T> where Self: Sized {
    fn fatal(self) -> Result<T>;
    fn bad_request(self) -> Result<T>;
}

impl<T, E> ErrorExt<T> for std::result::Result<T, E> where E: std::error::Error {
    fn fatal(self) -> Result<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(Error::InternalError(err.to_string())),
        }
    }

    fn bad_request(self) -> Result<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(Error::BadRequest(err.to_string())),
        }
    }
}

