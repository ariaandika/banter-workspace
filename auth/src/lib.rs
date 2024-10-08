use std::fmt::{Debug, Display};

use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::debug;
use types::{Date, Deserialize, Serialize, UserId, Users, WhId, WhType};
use argon2::{password_hash::{Error::Password, Result as ArgonResult}, Argon2, PasswordHash, PasswordVerifier as _};

pub use types::Role;
pub const DUMMY_PASSWD: &str = "$argon2id$v=19$m=19456,t=2,p=1$jZlzXaKWE9bOcXz99qDobg$L8MH9ZkgV/gdIhWQ72tNhDhmX4gPkdlzIUNfIF2oO4k";

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub user_id: UserId,
    pub name: String,
    pub phone: String,
    pub role: Role,
    pub role_data: Value,
    pub metadata: Value,
    pub created_at: Date,
    pub updated_at: Date,
    pub verified_at: Option<Date>,
    pub signed_at: Date,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesData {
    pub wh_id: WhId,
    pub wh_name: String,
    pub wh_type: WhType,
}

impl Token {
    pub fn new(user: Users, role_data: Value) -> Self {
        Self {
            user_id: user.user_id,
            name: user.name,
            phone: user.phone,
            role: user.role,
            role_data,
            metadata: user.metadata,
            created_at: user.created_at,
            updated_at: user.updated_at,
            verified_at: user.verified_at,
            signed_at: Default::default()
        }
    }

    pub fn from_token_str(secret: &str, token_str: &str) -> Result<Self> {
        let Some(body) = sign::verify(secret, token_str) else {
            debug!(target: "login failed","invalid hmac");
            return Err(Error::Unauthorized);
        };

        match serde_json::from_str(&body) {
            Ok(token) => Ok(token),
            Err(error) => {
                tracing::error!(target: "assertion failed", %error, "token deserialization");
                Err(Error::InvalidToken)
            },
        }
    }

    pub fn split<T>(mut self) -> Result<(Self, T)> where T: DeserializeOwned {
        match serde_json::from_value(self.role_data.take()) {
            Ok(role_data) => Ok((self,role_data)),
            Err(error) => {
                tracing::error!(target: "assertion failed", %error, "role data deserialization");
                Err(Error::InvalidToken)
            }
        }
    }
}

/// This is cpu bound process, call it with [`tokio::task::spawn_blocking`]
///
/// # Error
/// hashing error, its considered as internal error,
/// because hashed password is not user provided value,
/// but its from database
pub fn verify_passwd(password: &str, hashed: &str) -> ArgonResult<Option<()>> {
    if cfg!(test) { return Ok((password == hashed).then_some(())) }

    let parsed_hash = PasswordHash::new(&hashed)?;
    let passwd_ok = Argon2::default().verify_password(password.as_bytes(), &parsed_hash);

    match passwd_ok {
        Ok(ok) => Ok(Some(ok)),
        Err(Password) => Ok(None),
        Err(error) => {
            tracing::error!(target: "assertion failed", %error, "password verification");
            Err(error)
        },
    }
}

/// This is cpu bound process, call it with [`tokio::task::spawn_blocking`]
///
/// this is used to prevent timing attack, when the user not found in database
pub fn mock_verify(password: &str) -> ArgonResult<()> {
    if cfg!(test) { return Ok(()) }

    let parsed_hash = PasswordHash::new(DUMMY_PASSWD)?;
    let passwd_ok = Argon2::default().verify_password(password.as_bytes(), &parsed_hash);

    match passwd_ok {
        Err(Password) => Ok(()),
        e => e
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    Unauthorized,
    InvalidCredential,
    Forbidden,
    InvalidToken
}

impl Error {
    #[inline]
    pub const fn error(&self) -> &'static str {
        match self {
            Error::Unauthorized => "Unauthorized",
            Error::InvalidCredential => "Invalid Credential",
            Error::Forbidden => "Forbidden",
            Error::InvalidToken => "Invalid Token",
        }
    }
    pub const fn message(&self) -> &'static str {
        match self {
            Error::Unauthorized => "Authentication Required",
            Error::InvalidCredential => "Invalid phone or password",
            Error::Forbidden => "You are not allowed to access this resource",
            Error::InvalidToken => "Token invalid, please issue a new token",
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

pub mod sign {
    use sha2::Sha256;
    use hmac::{Hmac, Mac};
    use base64::prelude::*;

    type Sign = Hmac<Sha256>;

    pub fn sign(key: &str, msg: &str) -> String {
        let msg = to_base(msg);
        let mut mac = Sign::new_from_slice(key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(msg.as_bytes());
        msg + "." + &to_base(mac.finalize().into_bytes())
    }

    pub fn verify(key: &str, value: &str) -> Option<String> {
        let (msg, signature) = value.split_once(".")?;
        let mut mac = Sign::new_from_slice(key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(msg.as_bytes());
        mac.verify_slice(&from_base(signature)?).ok()?;
        String::from_utf8(from_base(msg)?).ok()
    }

    pub fn to_base<T>(value: T) -> String where T: AsRef<[u8]>, {
        BASE64_URL_SAFE_NO_PAD.encode(value)
    }

    pub fn from_base(value: &str) -> Option<Vec<u8>> {
        BASE64_URL_SAFE_NO_PAD.decode(value).ok()
    }
}
