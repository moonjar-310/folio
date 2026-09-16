use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use hmac::{Hmac, Mac};

pub const ACCESS_SECONDS: u64 = 300;
const HEADER: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub username: String,
    pub provider: String,
    pub csrf: String,
    pub iat: u64,
    pub exp: u64,
}
fn mac(secret: &str) -> Result<Hmac<Sha256>> {
    if secret.len() < 64 {
        return Err(fail(503, "Configure a strong FOLIO_JWT_SECRET."));
    }
    Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| fail(503, "Invalid signing key"))
}
pub fn sign(secret: &str, claims: &Claims) -> Result<String> {
    let payload = serde_json::to_vec(claims).map_err(|_| fail(500, "Invalid claims"))?;
    let data = format!("{HEADER}.{}", B64.encode(payload));
    let mut signer = mac(secret)?;
    signer.update(data.as_bytes());
    Ok(format!(
        "{data}.{}",
        B64.encode(signer.finalize().into_bytes())
    ))
}
pub fn verify(secret: &str, token: &str, now: u64, provider: &str) -> Result<Claims> {
    let invalid = || fail(401, "Your access token expired. Please sign in again.");
    if token.len() > 4096 {
        return Err(invalid());
    }
    let (data, signature) = token.rsplit_once('.').ok_or_else(invalid)?;
    let (header, payload) = data.split_once('.').ok_or_else(invalid)?;
    // Only this exact HS256 JWT header is accepted: no algorithm negotiation.
    if header != HEADER {
        return Err(invalid());
    }
    let mut verifier = mac(secret)?;
    verifier.update(data.as_bytes());
    verifier
        .verify_slice(&B64.decode(signature).map_err(|_| invalid())?)
        .map_err(|_| invalid())?;
    let claims: Claims = serde_json::from_slice(&B64.decode(payload).map_err(|_| invalid())?)
        .map_err(|_| invalid())?;
    if claims.iss != "folio"
        || claims.aud != "folio-api"
        || claims.provider != provider
        || claims.sub.is_empty()
        || claims.username.is_empty()
        || claims.csrf.is_empty()
        || claims.iat > now
        || claims.exp <= now
        || claims.exp.checked_sub(claims.iat) != Some(ACCESS_SECONDS)
    {
        return Err(invalid());
    }
    Ok(claims)
}
