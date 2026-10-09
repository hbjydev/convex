use std::sync::Arc;

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::http::{HeaderMap, HeaderValue};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    auth_policy::AuthenticationConfig,
    http::{AppError, AppResult},
};

#[derive(Clone)]
pub struct SecurityConfig {
    pub origin: String,
    pub setup_token: Option<String>,
    pub secure_cookies: bool,
    pub mail: Option<Arc<Mail>>,
    pub authentication: AuthenticationConfig,
}

pub struct Mail {
    pub transport: AsyncSmtpTransport<Tokio1Executor>,
    pub from: lettre::message::Mailbox,
}

impl SecurityConfig {
    pub fn development() -> Self {
        Self {
            origin: "http://127.0.0.1:5173".into(),
            setup_token: None,
            secure_cookies: false,
            mail: None,
            authentication: AuthenticationConfig::default(),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let origin =
            std::env::var("CONVEX_PUBLIC_URL").unwrap_or_else(|_| "http://127.0.0.1:5173".into());
        let url: axum::http::Uri = origin.parse()?;
        anyhow::ensure!(
            url.path() == "/" && url.query().is_none() && url.authority().is_some(),
            "CONVEX_PUBLIC_URL must be an origin without a path or query"
        );
        let secure_cookies = url.scheme_str() == Some("https");
        anyhow::ensure!(
            secure_cookies
                || (url.scheme_str() == Some("http")
                    && matches!(url.host(), Some("127.0.0.1" | "localhost" | "[::1]"))),
            "HTTPS is required outside loopback development"
        );
        let setup_token = std::env::var("CONVEX_SETUP_TOKEN").ok();
        anyhow::ensure!(
            setup_token.as_ref().is_none_or(|t| t.len() >= 32),
            "CONVEX_SETUP_TOKEN must contain at least 32 characters"
        );
        let mail = if let Some(host) = std::env::var("CONVEX_SMTP_HOST")
            .ok()
            .filter(|host| !host.trim().is_empty())
        {
            let plain = std::env::var("CONVEX_SMTP_TLS").as_deref() == Ok("plain");
            anyhow::ensure!(
                !plain || matches!(host.as_str(), "localhost" | "127.0.0.1" | "mailpit"),
                "Plain SMTP is only supported for local development"
            );
            let builder = if plain {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            } else {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?
            };
            let mut builder = builder
                .port(
                    std::env::var("CONVEX_SMTP_PORT")
                        .unwrap_or_else(|_| if plain { "1025" } else { "587" }.into())
                        .parse()?,
                )
                .timeout(Some(std::time::Duration::from_secs(10)));
            if let Ok(username) = std::env::var("CONVEX_SMTP_USERNAME") {
                builder = builder.credentials(Credentials::new(
                    username,
                    std::env::var("CONVEX_SMTP_PASSWORD")?,
                ));
            }
            Some(Arc::new(Mail {
                transport: builder.build(),
                from: std::env::var("CONVEX_SMTP_FROM")?.parse()?,
            }))
        } else {
            None
        };
        Ok(Self {
            origin: origin.trim_end_matches('/').into(),
            setup_token,
            secure_cookies,
            mail,
            authentication: AuthenticationConfig::from_env()?,
        })
    }

    pub fn cookie(&self, token: &str) -> HeaderValue {
        format!(
            "convex_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{suffix}",
            if token.is_empty() {
                0
            } else {
                self.authentication.session_lifetime.seconds()
            },
            suffix = if self.secure_cookies { "; Secure" } else { "" }
        )
        .parse()
        .expect("generated cookie is ASCII")
    }

    pub async fn send(&self, to: &str, subject: &str, body: String) -> AppResult<bool> {
        let Some(mail) = &self.mail else {
            return Ok(false);
        };
        let message = Message::builder()
            .from(mail.from.clone())
            .to(to
                .parse()
                .map_err(|_| AppError::validation("Enter a valid email address."))?)
            .subject(subject)
            .body(body)
            .map_err(AppError::internal)?;
        mail.transport
            .send(message)
            .await
            .map_err(AppError::internal)?;
        Ok(true)
    }
}

pub fn token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}
pub fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
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
pub fn email(value: &str) -> AppResult<String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() > 254
        || !normalized.is_ascii()
        || normalized.chars().any(char::is_whitespace)
        || normalized.parse::<lettre::Address>().is_err()
    {
        return Err(AppError::validation("Enter a valid ASCII email address."));
    }
    Ok(normalized)
}
pub fn name(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 100 {
        return Err(AppError::validation(
            "Names must contain between 1 and 100 characters.",
        ));
    }
    Ok(value.into())
}
pub fn workspace_slug(value: &str) -> AppResult<String> {
    let value = value.trim().to_ascii_lowercase();
    if [
        "account",
        "api",
        "assets",
        "auth",
        "invite",
        "setup",
        "system",
        "workspaces",
    ]
    .contains(&value.as_str())
    {
        return Err(AppError::validation(
            "This workspace URL is reserved for the application. Choose another slug.",
        ));
    }
    if !(3..=63).contains(&value.len())
        || value.split('-').any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
    {
        return Err(AppError::validation(
            "Workspace URLs need 3–63 lowercase letters or numbers, separated by single hyphens.",
        ));
    }
    Ok(value)
}
pub fn team_key(value: &str) -> AppResult<String> {
    let value = value.trim().to_ascii_uppercase();
    if !(2..=10).contains(&value.len())
        || !value.as_bytes()[0].is_ascii_uppercase()
        || !value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return Err(AppError::validation(
            "Team keys need 2–10 letters or numbers, starting with a letter.",
        ));
    }
    Ok(value)
}
pub async fn hash_password(password: String) -> AppResult<String> {
    if !(12..=128).contains(&password.chars().count()) || password.len() > 512 {
        return Err(AppError::validation("Use a password of 12–128 characters."));
    }
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map(|h| h.to_string())
            .map_err(AppError::internal)
    })
    .await
    .map_err(AppError::internal)?
}
pub async fn verify_password(password: String, hash: String) -> AppResult<()> {
    let valid = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).is_ok_and(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
    })
    .await
    .map_err(AppError::internal)?;
    if valid {
        Ok(())
    } else {
        Err(AppError::unauthorized())
    }
}

/// Persisted fixed windows coordinate throttling across replicas. A global bucket also bounds
/// password-hashing work when an attacker varies email addresses.
pub async fn rate_limit(pool: &PgPool, subject: &str) -> AppResult<()> {
    for (bucket, maximum) in [("global".to_owned(), 120), (digest(subject), 10)] {
        let attempts: i32 = sqlx::query_scalar("INSERT INTO convex.auth_rate_limits (bucket) VALUES ($1) ON CONFLICT (bucket) DO UPDATE SET attempts = CASE WHEN convex.auth_rate_limits.started_at < now() - interval '15 minutes' THEN 1 ELSE convex.auth_rate_limits.attempts + 1 END, started_at = CASE WHEN convex.auth_rate_limits.started_at < now() - interval '15 minutes' THEN now() ELSE convex.auth_rate_limits.started_at END RETURNING attempts").bind(bucket).fetch_one(pool).await?;
        if attempts > maximum {
            return Err(AppError::rate_limited());
        }
    }
    sqlx::query("DELETE FROM convex.auth_rate_limits WHERE started_at < now() - interval '1 day'")
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Clone, Copy)]
pub enum LoginMethod {
    Local,
    Oidc(Uuid),
}

pub async fn session_method(
    tx: &mut Transaction<'_, Postgres>,
    headers: &HeaderMap,
) -> AppResult<LoginMethod> {
    let identity: Option<Uuid> =
        sqlx::query_scalar("SELECT oidc_identity_id FROM convex.sessions WHERE token_hash=$1")
            .bind(digest(&session_token(headers)?))
            .fetch_one(&mut **tx)
            .await?;
    Ok(identity.map_or(LoginMethod::Local, LoginMethod::Oidc))
}

pub async fn new_session(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    method: LoginMethod,
) -> AppResult<String> {
    let enabled: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM convex.users WHERE id=$1 AND disabled_at IS NULL)",
    )
    .bind(user)
    .fetch_one(&mut **tx)
    .await?;
    if !enabled {
        return Err(AppError::unauthorized());
    }
    let identity = match method {
        LoginMethod::Local => {
            crate::auth_policy::ensure_local_allowed(tx, user).await?;
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM convex.local_credentials WHERE user_id=$1)",
            )
            .bind(user)
            .fetch_one(&mut **tx)
            .await?;
            if !exists {
                return Err(AppError::unauthorized());
            }
            None
        }
        LoginMethod::Oidc(identity) => {
            let eligible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.oidc_identities i CROSS JOIN convex.authentication_policy p WHERE i.id=$1 AND i.user_id=$2 AND i.unlinked_at IS NULL AND p.oidc_enabled AND i.issuer=p.oidc_issuer)")
                .bind(identity).bind(user).fetch_one(&mut **tx).await?;
            if !eligible {
                return Err(AppError::unauthorized());
            }
            Some(identity)
        }
    };
    let token = token();
    sqlx::query("INSERT INTO convex.sessions (id,user_id,token_hash,login_method,oidc_identity_id,provider_revision,expires_at) SELECT $1,$2,$3,$4,$5,CASE WHEN $5::uuid IS NOT NULL THEN provider_revision END,now() + make_interval(secs => session_ttl_seconds::double precision) FROM convex.authentication_policy")
        .bind(Uuid::new_v4()).bind(user).bind(digest(&token))
        .bind(if identity.is_some() { "oidc" } else { "local" }).bind(identity)
        .execute(&mut **tx).await?;
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn password_and_validation() {
        let hash = hash_password("a long correct password".into())
            .await
            .unwrap();
        assert!(hash.starts_with("$argon2id$"));
        verify_password("a long correct password".into(), hash.clone())
            .await
            .unwrap();
        assert!(
            verify_password("incorrect password".into(), hash)
                .await
                .is_err()
        );
        assert!(hash_password("short".into()).await.is_err());
        assert_eq!(email(" Person@Example.COM ").unwrap(), "person@example.com");
        assert!(email("Name <person@example.com>").is_err());
        assert!(team_key("1ENG").is_err());
        assert_eq!(team_key("eng").unwrap(), "ENG");
        assert_eq!(token().len(), 64);
    }

    #[test]
    fn workspace_slugs_are_url_safe() {
        assert_eq!(workspace_slug(" Acme-2026 ").unwrap(), "acme-2026");
        assert!(workspace_slug(&"a".repeat(63)).is_ok());
        for invalid in [
            "",
            "ab",
            "-acme",
            "acme-",
            "acme--team",
            "acme/team",
            "a b",
            "café",
        ] {
            assert!(workspace_slug(invalid).is_err(), "accepted {invalid:?}");
        }
        assert!(workspace_slug(&"a".repeat(64)).is_err());
        for reserved in [
            "account",
            "api",
            "assets",
            "auth",
            "invite",
            "setup",
            "system",
            "workspaces",
        ] {
            assert!(workspace_slug(reserved).is_err());
        }
    }
}
