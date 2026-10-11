pub mod models;

use crate::{
    accounts::models::*,
    credentials::{Credentials, SessionUpdate},
};
use async_trait::async_trait;
use convex_core::Result;

/// Authentication and credential lifecycle; account/workspace coordination lives in convex-app.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait AccountAuthentication: Send + Sync {
    async fn bootstrap(&self) -> Result<BootstrapInfo>;
    async fn login(
        &self,
        credentials: Credentials,
        req: LoginRequest,
    ) -> Result<SessionUpdate<User>>;
    async fn logout(&self, credentials: Credentials) -> Result<SessionUpdate<MessageResponse>>;
    async fn change_password(
        &self,
        credentials: Credentials,
        req: PasswordChange,
    ) -> Result<SessionUpdate<MessageResponse>>;
    async fn request_recovery(&self, req: RecoveryRequest) -> Result<MessageResponse>;
    async fn reset_password(&self, req: ResetRequest) -> Result<SessionUpdate<MessageResponse>>;
    async fn sessions(&self, credentials: Credentials) -> Result<Vec<SessionInfo>>;
    async fn revoke_session(
        &self,
        credentials: Credentials,
        req: RevokeSession,
    ) -> Result<MessageResponse>;
    async fn admin_accounts(&self, credentials: Credentials) -> Result<Vec<AdminAccount>>;
}
