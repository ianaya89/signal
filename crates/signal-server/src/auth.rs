//! Subsonic query-string auth. Signal is single-user: the username is
//! required by the protocol but its value is ignored; only the shared
//! password matters. Runs on every endpoint including ping and binaries.

use md5::{Digest, Md5};

use crate::envelope::ApiError;
use crate::params::Params;

pub(crate) fn check(params: &Params, password: &str) -> Result<(), ApiError> {
    params.require("u")?;

    if password.is_empty() {
        // server should never start like this; refuse rather than allow all
        return Err(ApiError::wrong_credentials());
    }

    if let (Some(token), Some(salt)) = (params.get("t"), params.get("s")) {
        let mut hasher = Md5::new();
        hasher.update(password.as_bytes());
        hasher.update(salt.as_bytes());
        let expected = hex::encode(hasher.finalize());
        return if expected.eq_ignore_ascii_case(token) {
            Ok(())
        } else {
            Err(ApiError::wrong_credentials())
        };
    }

    if let Some(p) = params.get("p") {
        let presented = if let Some(hexed) = p.strip_prefix("enc:") {
            let bytes = hex::decode(hexed).map_err(|_| ApiError::wrong_credentials())?;
            String::from_utf8(bytes).map_err(|_| ApiError::wrong_credentials())?
        } else {
            p.to_owned()
        };
        return if presented == password {
            Ok(())
        } else {
            Err(ApiError::wrong_credentials())
        };
    }

    Err(ApiError::missing_param("t+s or p"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn token_for(password: &str, salt: &str) -> String {
        let mut hasher = Md5::new();
        hasher.update(password.as_bytes());
        hasher.update(salt.as_bytes());
        hex::encode(hasher.finalize())
    }

    #[test]
    fn token_auth_accepts_and_rejects() {
        let t = token_for("sesame", "c19b2d");
        let good = Params::parse(&format!("u=any&t={t}&s=c19b2d"));
        assert!(check(&good, "sesame").is_ok());

        let bad = Params::parse(&format!("u=any&t={t}&s=different"));
        assert!(check(&bad, "sesame").is_err());
    }

    #[test]
    fn plain_and_enc_password() {
        let plain = Params::parse("u=any&p=sesame");
        assert!(check(&plain, "sesame").is_ok());

        let enc = Params::parse(&format!("u=any&p=enc:{}", hex::encode("sesame")));
        assert!(check(&enc, "sesame").is_ok());

        let wrong = Params::parse("u=any&p=nope");
        assert!(check(&wrong, "sesame").is_err());
    }

    #[test]
    fn missing_user_or_creds_fails() {
        assert!(check(&Params::parse("p=sesame"), "sesame").is_err());
        assert!(check(&Params::parse("u=any"), "sesame").is_err());
    }

    #[test]
    fn empty_configured_password_always_fails() {
        let p = Params::parse("u=any&p=");
        assert!(check(&p, "").is_err());
    }
}
