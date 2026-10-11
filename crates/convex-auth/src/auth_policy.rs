//! Installation authentication policy. Activation coordinates replicas; request checks
//! use PostgreSQL alone and never depend on an identity provider being reachable.
use anyhow::{Context, Result, ensure};
use http::Uri;

use crate::security;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionLifetime(u32);

impl Default for SessionLifetime {
    fn default() -> Self {
        Self(604800)
    }
}

impl SessionLifetime {
    pub fn parse(value: &str) -> Result<Self> {
        ensure!(
            !value.is_empty() && value.bytes().all(|v| v.is_ascii_digit()),
            "CONVEX_SESSION_TTL_SECONDS must be a positive integer"
        );
        let seconds: u32 = value
            .parse()
            .context("CONVEX_SESSION_TTL_SECONDS is out of range")?;
        ensure!(
            (1..=i32::MAX as u32).contains(&seconds),
            "CONVEX_SESSION_TTL_SECONDS must be between 1 and 2147483647"
        );
        Ok(Self(seconds))
    }

    pub fn seconds(self) -> u32 {
        self.0
    }
}

// Deliberately no Debug implementation: client secrets must never enter logs.
#[derive(Clone)]
pub struct ProviderConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub display_name: String,
    pub enabled: bool,
}

#[derive(Clone, Default)]
pub struct AuthenticationConfig {
    pub session_lifetime: SessionLifetime,
    pub provider: Option<ProviderConfig>,
    pub oidc_required: bool,
}

impl AuthenticationConfig {
    pub fn from_env() -> Result<Self> {
        Self::read(|key| match std::env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(error).with_context(|| format!("{key} must be UTF-8")),
        })
    }

    fn read(mut get: impl FnMut(&str) -> Result<Option<String>>) -> Result<Self> {
        let session_lifetime = get("CONVEX_SESSION_TTL_SECONDS")?
            .map(|value| SessionLifetime::parse(&value))
            .transpose()?
            .unwrap_or_default();
        let oidc_required = boolean(get("CONVEX_OIDC_REQUIRED")?, false, "CONVEX_OIDC_REQUIRED")?;
        let issuer = get("CONVEX_OIDC_ISSUER_URL")?;
        let client_id = get("CONVEX_OIDC_CLIENT_ID")?;
        let client_secret = get("CONVEX_OIDC_CLIENT_SECRET")?;
        let display_name = get("CONVEX_OIDC_DISPLAY_NAME")?;
        let enabled_setting = get("CONVEX_OIDC_ENABLED")?;
        let has_provider = issuer.is_some() || client_id.is_some() || client_secret.is_some();
        let enabled = boolean(enabled_setting, has_provider, "CONVEX_OIDC_ENABLED")?;
        let provider = if has_provider {
            let issuer = issuer.context("CONVEX_OIDC_ISSUER_URL is required")?;
            let client_id = client_id.context("CONVEX_OIDC_CLIENT_ID is required")?;
            let client_secret = client_secret.context("CONVEX_OIDC_CLIENT_SECRET is required")?;
            validate_issuer(&issuer)?;
            ensure!(
                !client_id.trim().is_empty(),
                "CONVEX_OIDC_CLIENT_ID must not be empty"
            );
            ensure!(
                !client_secret.trim().is_empty(),
                "CONVEX_OIDC_CLIENT_SECRET must not be empty"
            );
            let display_name = display_name.unwrap_or_else(|| "Single sign-on".into());
            ensure!(
                !display_name.trim().is_empty() && display_name.chars().count() <= 100,
                "CONVEX_OIDC_DISPLAY_NAME must contain 1–100 characters"
            );
            Some(ProviderConfig {
                issuer,
                client_id,
                client_secret,
                display_name,
                enabled,
            })
        } else {
            ensure!(
                !enabled && display_name.is_none(),
                "OIDC settings require a complete provider configuration"
            );
            None
        };
        ensure!(
            !oidc_required || enabled,
            "Required OIDC needs a configured, enabled provider"
        );
        Ok(Self {
            session_lifetime,
            provider,
            oidc_required,
        })
    }

    pub fn provider_revision(&self) -> Option<String> {
        self.provider
            .as_ref()
            .filter(|p| p.enabled)
            .map(|p| hash(&(&p.issuer, &p.client_id)))
    }

    pub fn exchange_revision(&self) -> Option<String> {
        self.provider
            .as_ref()
            .filter(|p| p.enabled)
            .map(|p| hash(&(&p.issuer, &p.client_id, &p.client_secret)))
    }

    #[cfg(any(feature = "postgres", test))]
    pub(crate) fn fingerprint(&self) -> String {
        hash(&(
            self.session_lifetime.seconds(),
            self.oidc_required,
            self.provider.as_ref().map(|p| {
                (
                    &p.issuer,
                    &p.client_id,
                    &p.client_secret,
                    &p.display_name,
                    p.enabled,
                )
            }),
        ))
    }
}

fn hash(value: &impl serde::Serialize) -> String {
    security::digest(
        &serde_json::to_string(value).expect("authentication configuration is serializable"),
    )
}

fn boolean(value: Option<String>, default: bool, name: &str) -> Result<bool> {
    match value.as_deref() {
        None => Ok(default),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => anyhow::bail!("{name} must be true or false"),
    }
}

fn validate_issuer(issuer: &str) -> Result<()> {
    let url: Uri = issuer
        .parse()
        .context("CONVEX_OIDC_ISSUER_URL must be an absolute URL")?;
    ensure!(
        url.authority().is_some_and(|a| !a.as_str().contains('@'))
            && url.query().is_none()
            && !issuer.contains('#'),
        "CONVEX_OIDC_ISSUER_URL must have a host and no credentials, query, or fragment"
    );
    ensure!(
        url.scheme_str() == Some("https")
            || (url.scheme_str() == Some("http")
                && matches!(url.host(), Some("127.0.0.1" | "localhost" | "[::1]"))),
        "OIDC requires HTTPS outside loopback development"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(values: &[(&str, &str)]) -> Result<AuthenticationConfig> {
        AuthenticationConfig::read(|key| {
            Ok(values
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).into()))
        })
    }

    #[test]
    fn session_lifetime_and_provider_configuration_are_validated_without_io() {
        assert_eq!(config(&[]).unwrap().session_lifetime.seconds(), 604800);
        assert_eq!(SessionLifetime::parse("3600").unwrap().seconds(), 3600);
        assert!(SessionLifetime::parse("2147483647").is_ok());
        for value in [
            "",
            "0",
            "-1",
            "+1",
            "1.5",
            " 60",
            "2147483648",
            "999999999999999999999",
        ] {
            assert!(SessionLifetime::parse(value).is_err(), "accepted {value}");
        }
        for issuer in [
            "https://id.example/realms/convex",
            "http://127.0.0.1:8081/realms/convex",
            "http://[::1]:8081/realm",
        ] {
            assert!(validate_issuer(issuer).is_ok(), "rejected {issuer}");
        }
        for issuer in [
            "/relative",
            "http://id.example/realm",
            "https://user:secret@id.example",
            "https://id.example?x=1",
            "https://id.example#fragment",
        ] {
            assert!(validate_issuer(issuer).is_err(), "accepted {issuer}");
        }
        assert!(config(&[("CONVEX_OIDC_REQUIRED", "true")]).is_err());
        assert!(config(&[("CONVEX_OIDC_REQUIRED", "yes")]).is_err());
        assert!(config(&[("CONVEX_OIDC_CLIENT_ID", "client")]).is_err());
        let values = [
            ("CONVEX_OIDC_ISSUER_URL", "https://id.example/realm"),
            ("CONVEX_OIDC_CLIENT_ID", "client"),
            ("CONVEX_OIDC_CLIENT_SECRET", "secret"),
            ("CONVEX_OIDC_REQUIRED", "true"),
        ];
        let first = config(&values).unwrap();
        assert!(first.provider.as_ref().unwrap().enabled);
        let mut rotated = first.clone();
        rotated.provider.as_mut().unwrap().client_secret = "new secret".into();
        assert_eq!(first.provider_revision(), rotated.provider_revision());
        assert_ne!(first.exchange_revision(), rotated.exchange_revision());
        assert_ne!(first.fingerprint(), rotated.fingerprint());
        let mut disabled = values.to_vec();
        disabled.push(("CONVEX_OIDC_ENABLED", "false"));
        assert!(config(&disabled).is_err());
    }
}
