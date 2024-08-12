

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
