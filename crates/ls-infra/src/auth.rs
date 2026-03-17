use std::sync::LazyLock;

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{
        Error as PasswordHashError, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use rand::random;
use sha2::{Digest, Sha256};

const ARGON2_M_COST_KIB: u32 = 19_456;
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 1;
const ARGON2_OUTPUT_LEN: usize = 32;
const ARGON2ID_ALGORITHM: &str = "argon2id";

static PASSWORD_HASHER: LazyLock<Argon2> = LazyLock::new(|| {
    let params = Params::new(
        ARGON2_M_COST_KIB,
        ARGON2_T_COST,
        ARGON2_P_COST,
        Some(ARGON2_OUTPUT_LEN),
    )
    .expect("static argon2 parameters should be valid");

    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordVerifyOutcome {
    Verified,
    Invalid,
    ResetRequired,
}

pub fn hash_password(password: &str) -> String {
    let salt_bytes: [u8; 16] = random();
    let salt = SaltString::encode_b64(&salt_bytes).expect("salt encoding should not fail");
    PASSWORD_HASHER
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hashing should not fail with valid static parameters")
        .to_string()
}

pub fn verify_password(password: &str, stored_hash: &str) -> PasswordVerifyOutcome {
    let parsed_hash = match PasswordHash::new(stored_hash) {
        Ok(hash) => hash,
        Err(_) => return PasswordVerifyOutcome::ResetRequired,
    };

    if parsed_hash.algorithm.as_str() != ARGON2ID_ALGORITHM {
        return PasswordVerifyOutcome::ResetRequired;
    }

    match PASSWORD_HASHER.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => PasswordVerifyOutcome::Verified,
        Err(PasswordHashError::Password) => PasswordVerifyOutcome::Invalid,
        Err(_) => PasswordVerifyOutcome::ResetRequired,
    }
}

pub fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn new_access_token() -> String {
    let bytes: [u8; 32] = random();
    format!("ls_{}", URL_SAFE_NO_PAD.encode(bytes))
}

pub fn new_admin_api_key(prefix: &str) -> String {
    let bytes: [u8; 32] = random();
    let random_suffix = URL_SAFE_NO_PAD.encode(bytes);

    let prefix = prefix.trim();
    if prefix.is_empty() {
        return format!("lsadm_{}", random_suffix);
    }
    format!("{}_{}", prefix, random_suffix)
}

pub fn expires_at(ttl_hours: i64) -> DateTime<Utc> {
    Utc::now() + Duration::hours(ttl_hours)
}

#[cfg(test)]
mod tests {
    use super::{PasswordVerifyOutcome, hash_password, verify_password};
    use sha2::{Digest, Sha256};

    #[test]
    fn hash_password_creates_argon2id_hash_and_verifies() {
        let hash = hash_password("top-secret");
        assert!(hash.starts_with("$argon2id$"));
        assert_eq!(
            verify_password("top-secret", &hash),
            PasswordVerifyOutcome::Verified
        );
    }

    #[test]
    fn verify_password_rejects_legacy_sha256_hash() {
        let legacy = hex::encode(Sha256::digest("legacy-pass".as_bytes()));
        assert_eq!(
            verify_password("legacy-pass", &legacy),
            PasswordVerifyOutcome::ResetRequired
        );
    }

    #[test]
    fn verify_password_rejects_wrong_password_for_argon2id_hash() {
        let hash = hash_password("correct-password");
        assert_eq!(
            verify_password("wrong-password", &hash),
            PasswordVerifyOutcome::Invalid
        );
    }

    #[test]
    fn verify_password_accepts_reinitialized_hash_after_legacy_cutover() {
        let legacy = hex::encode(Sha256::digest("legacy-pass".as_bytes()));
        assert_eq!(
            verify_password("legacy-pass", &legacy),
            PasswordVerifyOutcome::ResetRequired
        );

        let reinitialized = hash_password("reinitialized-pass");
        assert_eq!(
            verify_password("reinitialized-pass", &reinitialized),
            PasswordVerifyOutcome::Verified
        );
    }
}
