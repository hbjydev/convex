pub mod models;

use crate::accounts::models::*;
use async_trait::async_trait;
use convex_auth::accounts::models::*;
use convex_auth::credentials::{Credentials, SessionUpdate};
use convex_core::Result;

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait Accounts: Send + Sync {
    async fn bootstrap(&self) -> Result<BootstrapInfo>;
    async fn setup(&self, req: SetupRequest) -> Result<SessionUpdate<User>>;
    async fn login(
        &self,
        credentials: Credentials,
        req: LoginRequest,
    ) -> Result<SessionUpdate<User>>;
    async fn me(&self, credentials: Credentials) -> Result<AccountResponse>;
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
    async fn disable(
        &self,
        credentials: Credentials,
        req: DisableRequest,
    ) -> Result<MessageResponse>;
    async fn admin_accounts(&self, credentials: Credentials) -> Result<Vec<AdminAccount>>;
}

#[derive(Clone)]
pub struct AccountApplication {
    pub control_plane: crate::control_plane::ControlPlane,
    pub authentication: std::sync::Arc<dyn convex_auth::accounts::AccountAuthentication>,
}

#[async_trait]
impl Accounts for AccountApplication {
    async fn bootstrap(&self) -> Result<BootstrapInfo> {
        self.authentication.bootstrap().await
    }
    async fn setup(&self, req: SetupRequest) -> Result<SessionUpdate<User>> {
        crate::account_workflows::setup(self.control_plane.clone(), req).await
    }
    async fn login(
        &self,
        credentials: Credentials,
        req: LoginRequest,
    ) -> Result<SessionUpdate<User>> {
        self.authentication.login(credentials, req).await
    }
    async fn me(&self, credentials: Credentials) -> Result<AccountResponse> {
        crate::account_workflows::me(self.control_plane.clone(), credentials).await
    }
    async fn logout(&self, credentials: Credentials) -> Result<SessionUpdate<MessageResponse>> {
        self.authentication.logout(credentials).await
    }
    async fn change_password(
        &self,
        credentials: Credentials,
        req: PasswordChange,
    ) -> Result<SessionUpdate<MessageResponse>> {
        self.authentication.change_password(credentials, req).await
    }
    async fn request_recovery(&self, req: RecoveryRequest) -> Result<MessageResponse> {
        self.authentication.request_recovery(req).await
    }
    async fn reset_password(&self, req: ResetRequest) -> Result<SessionUpdate<MessageResponse>> {
        self.authentication.reset_password(req).await
    }
    async fn sessions(&self, credentials: Credentials) -> Result<Vec<SessionInfo>> {
        self.authentication.sessions(credentials).await
    }
    async fn revoke_session(
        &self,
        credentials: Credentials,
        req: RevokeSession,
    ) -> Result<MessageResponse> {
        self.authentication.revoke_session(credentials, req).await
    }
    async fn disable(
        &self,
        credentials: Credentials,
        req: DisableRequest,
    ) -> Result<MessageResponse> {
        crate::account_workflows::disable(self.control_plane.clone(), credentials, req).await
    }
    async fn admin_accounts(&self, credentials: Credentials) -> Result<Vec<AdminAccount>> {
        self.authentication.admin_accounts(credentials).await
    }
}
