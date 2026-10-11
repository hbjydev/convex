use crate::http::{AppError, AppResult};
use axum::http::{HeaderMap, HeaderValue};
pub use convex_auth::security::digest;
use convex_auth::{credentials::Credentials, security::SecurityConfig};

#[derive(Clone)]
pub struct CookieConfig {
    pub origin: String,
    pub secure: bool,
    pub session_lifetime_seconds: u32,
}
impl From<&SecurityConfig> for CookieConfig {
    fn from(configuration: &SecurityConfig) -> Self {
        Self {
            origin: configuration.origin.clone(),
            secure: configuration.secure_cookies,
            session_lifetime_seconds: configuration.authentication.session_lifetime.seconds(),
        }
    }
}
impl CookieConfig {
    pub fn session_cookie(&self, token: &str) -> HeaderValue {
        self.cookie(
            "convex_session",
            token,
            if token.is_empty() {
                0
            } else {
                self.session_lifetime_seconds
            },
        )
    }
    pub fn proof_cookie(&self, token: &str) -> HeaderValue {
        self.cookie(
            "convex_method_proof",
            token,
            if token.is_empty() { 0 } else { 300 },
        )
    }
    pub fn browser_cookie(&self, token: &str) -> HeaderValue {
        self.cookie("convex_oidc_browser", token, 600)
    }
    fn cookie(&self, name: &str, token: &str, lifetime: u32) -> HeaderValue {
        format!(
            "{name}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={lifetime}{}",
            if self.secure { "; Secure" } else { "" }
        )
        .parse()
        .expect("generated cookie is ASCII")
    }
}

pub fn session_token(headers: &HeaderMap) -> AppResult<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| {
            h.split(';')
                .find_map(|v| v.trim().strip_prefix("convex_session="))
        })
        .filter(|t| t.len() == 64)
        .map(str::to_owned)
        .ok_or_else(AppError::unauthorized)
}
fn unique_token(headers: &HeaderMap, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    let mut values = headers
        .get_all("cookie")
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(';'))
        .filter_map(|v| v.trim().strip_prefix(&prefix));
    let value = values.next()?;
    if values.next().is_some() || value.len() != 64 || !value.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return None;
    }
    Some(value.into())
}
pub fn credentials(headers: &HeaderMap) -> Credentials {
    Credentials {
        session: session_token(headers).ok(),
        browser: unique_token(headers, "convex_oidc_browser"),
        method_proof: unique_token(headers, "convex_method_proof"),
    }
}
