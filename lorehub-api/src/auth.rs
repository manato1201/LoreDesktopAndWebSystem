use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::http::HeaderMap;
use rand::Rng;

pub const SESSION_COOKIE: &str = "lorehub_token";
pub const REFRESH_COOKIE: &str = "lorehub_refresh";

/// Access token server-side TTL, in seconds (30 minutes). Kept in sync with
/// the `Max-Age` on `session_cookie` so the browser-side expiry and the
/// server-side `SessionEntry::expires_at` never disagree.
pub const ACCESS_TOKEN_TTL_SECS: i64 = 30 * 60;

/// Refresh token server-side TTL, in seconds (7 days) — this matches the
/// `Max-Age` the single-token model used before dual tokens existed.
pub const REFRESH_TOKEN_TTL_SECS: i64 = 7 * 24 * 60 * 60;

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hashing a short demo password should never fail")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn generate_token() -> String {
    let bytes: [u8; 24] = rand::thread_rng().r#gen();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Generates a 40-char lowercase-hex fake commit SHA (20 random bytes,
/// hex-encoded) for the simulated commit endpoint — there is no real
/// content-hashing VCS backing this demo server.
pub fn generate_commit_hash() -> String {
    let bytes: [u8; 20] = rand::thread_rng().r#gen();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Generalized cookie-value extractor — pulls the value of the cookie named
/// `name` out of the request's `Cookie` header, or `None` if it isn't
/// present. Backs both `extract_session_token` (access token) and
/// `extract_refresh_token` (refresh token).
pub fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&format!("{name}=")) {
            return Some(value.to_string());
        }
    }
    None
}

pub fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    extract_cookie(headers, SESSION_COOKIE)
}

pub fn extract_refresh_token(headers: &HeaderMap) -> Option<String> {
    extract_cookie(headers, REFRESH_COOKIE)
}

/// Current time as unix seconds, used to stamp/compare `SessionEntry::expires_at`.
pub fn current_unix_time() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the unix epoch")
        .as_secs() as i64
}

pub fn session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={ACCESS_TOKEN_TTL_SECS}"
    )
}

pub fn cleared_session_cookie() -> String {
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

pub fn refresh_cookie(token: &str) -> String {
    format!(
        "{REFRESH_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={REFRESH_TOKEN_TTL_SECS}"
    )
}

pub fn cleared_refresh_cookie() -> String {
    format!("{REFRESH_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}
