use std::{borrow::Cow, env::var, fmt::{Debug, Display, Formatter as Fmt, Result as FmtRes}, future::Future, sync::LazyLock};
use auth::{Error as AuthError, Role, Token};
use bytes::Bytes;
use http_body_util::{BodyExt as _, Full};
use hyper::{header::{AUTHORIZATION, CONTENT_TYPE, COOKIE}, http::response::Builder as ResponseBuilder, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

pub use hyper::{body::Incoming as Body, http::request::Parts};
pub use serde_json::{json, ser};

pub type Request = hyper::Request<Body>;
pub type Response<T = Full<Bytes>> = hyper::Response<T>;
pub type Result<T = Response, E = Error> = std::result::Result<T,E>;
pub type Paginate = (u32,u32);

pub const NOT_FOUND: Result = Err(Error::Http(StatusCode::NOT_FOUND));
pub const UNAUTHORIZED: Result = Err(Error::Auth(AuthError::Unauthorized));
pub const GET: &Method = &Method::GET;
pub const POST: &Method = &Method::POST;

pub enum LogicError {
    UserIdNotFound(i32)
}

pub enum Error {
    Http(StatusCode),
    BadRequest(String),
    InternalError(String),
    Auth(AuthError),
    Logic(LogicError),
}

impl Error {
    #[deprecated = "into_response is user responsibility"]
    pub fn into_response(self) -> Response {
        if let Error::InternalError(ref message) = self {
            tracing::error!(target: "InternalError",message);
        }

        let build = Response::builder().status(self.status());

        // TODO: write body based on accept header
        build.empty().expect("Infallible")
    }

    #[inline]
    pub fn status(&self) -> StatusCode {
        match self {
            Error::Http(st) => st.clone(),
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Auth(AuthError::Forbidden) => StatusCode::FORBIDDEN,
            Error::Auth(_) => StatusCode::UNAUTHORIZED,
            Error::Logic(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    #[inline]
    pub const fn error(&self) -> &'static str {
        match self {
            Error::Http(s) => status_msg(s),
            Error::BadRequest(_) => "Bad Request",
            Error::InternalError(_) => "Internal Server Error",
            Error::Auth(er) => er.error(),
            Error::Logic(_) => "Unprocessable Entity",
        }
    }

    #[inline]
    #[doc = "InternalError message redaction is user responsibility"]
    pub fn message(self) -> String {
        match self {
            Error::Http(ref s) => status_msg(s).into(),
            Error::Auth(er) => er.message().into(),
            Error::BadRequest(m) | Error::InternalError(m) => m,
            Error::Logic(e) => match e {
                LogicError::UserIdNotFound(id) => format!("User Id `{id}` Not Found "),
            },
        }
    }

    pub fn message_write(&self, f: &mut Fmt<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Error::BadRequest(m) | Error::InternalError(m) => &*m,
            Error::Auth(e) => e.message(),
            e => e.error(),
        })
    }
}

pub trait BodyExt {
    fn json<T>(self) -> impl Future<Output = Result<T>> + Send where T: DeserializeOwned;
}

impl BodyExt for Body {
    async fn json<T>(self) -> Result<T> where T: DeserializeOwned {
        serde_json::from_slice(&self.collect().await?.to_bytes()).bad_request()
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
    fn json_str(&self) -> Result<String>;
}

impl<S> IntoResponse for S where S: Serialize {
    #[inline]
    fn into_response(self) -> Result { Response::builder().json(self) }
    fn json_str(&self) -> Result<String> { ser::to_string(self).map_err(|e|Error::BadRequest(e.to_string())) }
}

const SESSION_KEY: &str = "access_token";
const JWT_SECRET: LazyLock<String> = LazyLock::new(||var("JWT_SECRET").expect("unchecked jwt secret"));

pub trait PartsExt<'r> {
    fn normalize_path(&'r self) -> &'r str;
    fn normalize_prefix(&'r self, prefix: &'r str) -> &'r str;
    fn parse_query(&'r self) -> Paginate;
    fn get_cookie(&'r self, key: &str) -> Option<&'r str>;
    fn auth_header(&'r self) -> Option<&'r str>;
    fn get_session(&'r self) -> Result<Token>;
    fn get_session_role(&'r self, role: Role) -> Result<Token>;
}

impl<'r> PartsExt<'r> for Parts {
    fn normalize_path(&'r self) -> &'r str {
        self.uri.path().strip_suffix("/").unwrap_or(self.uri.path())
    }

    fn normalize_prefix(&'r self, prefix: &'r str) -> &'r str {
        let p = self.normalize_path();
        p.strip_prefix(prefix).unwrap_or(p)
    }

    fn parse_query(&'r self) -> Paginate {
        fn par((_, v): (Cow<str>,Cow<str>)) -> Option<u32> { v.parse().ok() }
        let Some(q) = self.uri.query() else { return (20,0); };
        let mut qs = form_urlencoded::parse(q.as_bytes());
        let limit = qs.find(|(k,_)|k=="limit").and_then(par).unwrap_or(20);
        let page = qs.find(|(k,_)|k=="page").and_then(par).unwrap_or(1);
        (limit, limit * page.checked_sub(1).unwrap_or(page))
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
impl Debug for Error { fn fmt(&self, f: &mut Fmt<'_>) -> FmtRes { self.message_write(f) } }
impl Display for Error { fn fmt(&self, f: &mut Fmt<'_>) -> FmtRes { write!(f, "{}", self.error()) } }
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

const fn status_msg(status: &StatusCode) -> &'static str {
    match *status {
        StatusCode::BAD_REQUEST => "Bad Request",
        StatusCode::UNAUTHORIZED => "Unauthorized",
        StatusCode::FORBIDDEN => "Forbidden",
        StatusCode::PAYLOAD_TOO_LARGE => "Payload Too Large",
        StatusCode::UNPROCESSABLE_ENTITY => "Unprocessable Entity",
        StatusCode::INTERNAL_SERVER_ERROR => "Internal Server Error",
        _ => "Http Error",
    }
}

