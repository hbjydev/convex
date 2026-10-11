use convex_ids::OidcIdentityId;
use std::sync::Arc;
use uuid::Uuid;

use crate::credentials::Credentials;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials as SmtpCredentials,
};
use sha2::{Digest, Sha256};

use crate::auth_policy::AuthenticationConfig;
use convex_core::{Error as AppError, Result as AppResult};

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
        let url: http::Uri = origin.parse()?;
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
            if let Some(username) = std::env::var("CONVEX_SMTP_USERNAME")
                .ok()
                .filter(|username| !username.trim().is_empty())
            {
                builder = builder.credentials(SmtpCredentials::new(
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

    pub async fn send(
        &self,
        to: &str,
        subject: &str,
        body: crate::email::Email,
    ) -> AppResult<bool> {
        let Some(mail) = &self.mail else {
            return Ok(false);
        };
        let message = Message::builder()
            .from(mail.from.clone())
            .to(to
                .parse()
                .map_err(|_| AppError::validation("Enter a valid email address."))?)
            .subject(subject)
            .multipart(body.into_multipart())
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
pub fn session_token(credentials: &Credentials) -> AppResult<String> {
    credentials.session_token().map(str::to_owned)
}
pub use convex_core::validation::name;

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

#[derive(Clone, Copy)]
pub enum LoginMethod {
    Local,
    Oidc(OidcIdentityId),
}

/// Used for absent accounts so failed local login still performs password verification.
pub fn dummy_password_hash() -> String {
    static HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        Argon2::default()
            .hash_password(
                b"dummy verification credential",
                &SaltString::generate(&mut OsRng),
            )
            .expect("default hash parameters")
            .to_string()
    });
    HASH.clone()
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
        assert_eq!(token().len(), 64);
    }
}
