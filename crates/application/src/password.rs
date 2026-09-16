use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use sha2::Sha256;
use subtle::ConstantTimeEq;

fn engine() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19 * 1024, 2, 1, Some(32)).expect("valid Argon2 parameters"),
    )
}

pub(crate) fn hash(password: &str) -> Result<String, String> {
    let salt =
        SaltString::encode_b64(uuid::Uuid::new_v4().as_bytes()).map_err(|e| e.to_string())?;
    engine()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// Returns whether a successful verification needs migration from legacy PBKDF2.
pub(crate) fn verify(password: &str, salt: &str, stored: &str) -> Option<bool> {
    if stored.starts_with("$argon2id$") {
        let parsed = PasswordHash::new(stored).ok()?;
        return engine()
            .verify_password(password.as_bytes(), &parsed)
            .ok()
            .map(|_| false);
    }
    if stored.len() != 64 || !stored.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut output = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), 600_000, &mut output);
    let actual: String = output.iter().map(|b| format!("{b:02x}")).collect();
    bool::from(actual.as_bytes().ct_eq(stored.as_bytes())).then_some(true)
}
