use types::{Serialize, Deserialize, UserId, Date, Role};
use argon2::{password_hash::{Error::Password, Result as ArgonResult}, Argon2, PasswordHash, PasswordVerifier as _};

pub const DUMMY_PASSWD: &str = "$argon2id$v=19$m=19456,t=2,p=1$jZlzXaKWE9bOcXz99qDobg$L8MH9ZkgV/gdIhWQ72tNhDhmX4gPkdlzIUNfIF2oO4k";

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub user_id: UserId,
    pub name: String,
    pub phone: String,
    pub role: Role,
    pub created_at: Date,
    pub updated_at: Date,
    pub verified_at: Option<Date>,
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
        Err(err) => Err(err),
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
