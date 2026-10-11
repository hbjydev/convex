use anyhow::Result;
use async_trait::async_trait;
use convex_ids::UserId;

/// Session lookup for issue operations, without account/session write locks.
/// The caller holds a shared workspace lock until its unit of work finishes.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait SessionRepository: Send {
    async fn new_session(
        &mut self,
        user: UserId,
        method: crate::security::LoginMethod,
    ) -> convex_core::Result<String>;
    async fn session_method(
        &mut self,
        credentials: crate::credentials::Credentials,
    ) -> convex_core::Result<crate::security::LoginMethod>;
    async fn revoke_all(&mut self, user: UserId) -> convex_core::Result<()>;
    async fn active_user(&mut self, token_digest: &str) -> Result<Option<UserId>>;
}

/// Confirms this replica still holds the active installation authentication policy.
/// HTTP middleware checks this before protected and authentication operations.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait AuthenticationPolicy: Send + Sync {
    async fn check(&self) -> Result<()>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait AccountRepository: Send {
    async fn contact_email(&mut self, user: UserId) -> convex_core::Result<Option<String>>;
    async fn authenticate(
        &mut self,
        credentials: crate::credentials::Credentials,
    ) -> convex_core::Result<crate::accounts::models::User>;
    async fn create_local(
        &mut self,
        email: String,
        hash: String,
        display_name: String,
        admin: bool,
    ) -> convex_core::Result<crate::accounts::models::User>;
    async fn find_for_update(
        &mut self,
        user: UserId,
    ) -> convex_core::Result<crate::accounts::models::User>;
    async fn protect_enabled_administrators(&mut self, user: UserId) -> convex_core::Result<()>;
    async fn set_disabled(&mut self, user: UserId, disabled: bool) -> convex_core::Result<()>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait InstallationRepository: Send {
    async fn initialized(&mut self) -> convex_core::Result<bool>;
    async fn mark_initialized(&mut self) -> convex_core::Result<()>;
}

/// Throttling uses independent, persisted windows before expensive credential work.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check(&self, subject: &str) -> convex_core::Result<()>;
}
