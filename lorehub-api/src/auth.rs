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

/// `true` unless the operator has explicitly opted out via
/// `LOREHUB_INSECURE_COOKIES=true` (or `1`). Named so that *forgetting* to
/// set the env var is the secure choice — cookies ship with `Secure` by
/// default, and only local plain-`http://` dev needs the explicit opt-out
/// (browsers refuse to store a `Secure` cookie set over a non-HTTPS
/// response, so without this opt-out `cargo run` over `http://localhost`
/// would silently break login).
///
/// Read once and cached in a `OnceLock` — the env var is startup-time
/// configuration, not something that changes mid-process, so there's no
/// reason to pay a `std::env::var` lookup on every single cookie-bearing
/// response.
fn cookies_secure() -> bool {
    static SECURE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SECURE.get_or_init(|| {
        let insecure_opt_out = std::env::var("LOREHUB_INSECURE_COOKIES")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        !insecure_opt_out
    })
}

/// Builds a single `Set-Cookie` value. Pulled out of the four public
/// builders below and parameterized on `secure` (rather than reaching for
/// [`cookies_secure`] itself) so unit tests can exercise both the
/// `Secure`-present and `Secure`-absent shapes directly, without touching
/// the process-global `LOREHUB_INSECURE_COOKIES` env var — env vars are
/// process-wide and `cookies_secure`'s `OnceLock` latches on first read, so
/// mutating the env var from a test would race every other test in the same
/// binary that also builds a cookie (which is effectively all of them, via
/// the `login` helper). Testing this pure string-building function in
/// isolation sidesteps that entirely.
fn cookie_string(name: &str, value: &str, max_age: i64, secure: bool) -> String {
    let secure_attr = if secure { "; Secure" } else { "" };
    format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax{secure_attr}; Max-Age={max_age}")
}

pub fn session_cookie(token: &str) -> String {
    cookie_string(
        SESSION_COOKIE,
        token,
        ACCESS_TOKEN_TTL_SECS,
        cookies_secure(),
    )
}

pub fn cleared_session_cookie() -> String {
    cookie_string(SESSION_COOKIE, "", 0, cookies_secure())
}

pub fn refresh_cookie(token: &str) -> String {
    cookie_string(
        REFRESH_COOKIE,
        token,
        REFRESH_TOKEN_TTL_SECS,
        cookies_secure(),
    )
}

pub fn cleared_refresh_cookie() -> String {
    cookie_string(REFRESH_COOKIE, "", 0, cookies_secure())
}

#[cfg(test)]
mod tests {
    use super::cookie_string;

    #[test]
    fn cookie_string_includes_secure_when_requested() {
        let cookie = cookie_string("lorehub_token", "abc123", 1800, true);
        assert_eq!(
            cookie,
            "lorehub_token=abc123; Path=/; HttpOnly; SameSite=Lax; Secure; Max-Age=1800"
        );
    }

    #[test]
    fn cookie_string_omits_secure_when_not_requested() {
        let cookie = cookie_string("lorehub_token", "abc123", 1800, false);
        assert_eq!(
            cookie,
            "lorehub_token=abc123; Path=/; HttpOnly; SameSite=Lax; Max-Age=1800"
        );
        assert!(!cookie.contains("Secure"));
    }

    #[test]
    fn cookie_string_cleared_variant_has_zero_max_age() {
        let secure = cookie_string("lorehub_refresh", "", 0, true);
        assert!(secure.starts_with("lorehub_refresh=; "));
        assert!(secure.ends_with("Max-Age=0"));
        assert!(secure.contains("; Secure"));

        let insecure = cookie_string("lorehub_refresh", "", 0, false);
        assert!(!insecure.contains("Secure"));
        assert!(insecure.ends_with("Max-Age=0"));
    }
}
