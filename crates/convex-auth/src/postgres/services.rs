use super::Context;
use crate::{accounts::models::*, credentials::*, methods_model::*, oidc_model::StartRequest};
use async_trait::async_trait;
use convex_core::Result;

#[derive(Clone)]
pub struct Authentication {
    pub context: Context,
}

#[async_trait]
impl crate::methods::LoginMethods for Authentication {
    async fn list(&self, credentials: Credentials) -> Result<Methods> {
        super::methods::list(self.context.clone(), credentials).await
    }
    async fn link(&self, credentials: Credentials, req: LocalProof) -> Result<Authorization> {
        super::methods::link(self.context.clone(), credentials, req).await
    }
    async fn prove(&self, credentials: Credentials, req: ProveOidc) -> Result<Authorization> {
        super::methods::prove(self.context.clone(), credentials, req).await
    }
    async fn add_local(
        &self,
        credentials: Credentials,
        req: AddLocal,
    ) -> Result<SessionUpdate<MessageResponse>> {
        super::methods::add_local(self.context.clone(), credentials, req).await
    }
    async fn remove_local(
        &self,
        credentials: Credentials,
    ) -> Result<SessionUpdate<MessageResponse>> {
        super::methods::remove_local(self.context.clone(), credentials).await
    }
    async fn remove_oidc(
        &self,
        credentials: Credentials,
        req: RemoveOidc,
    ) -> Result<SessionUpdate<MessageResponse>> {
        super::methods::remove_oidc(self.context.clone(), credentials, req).await
    }
}

#[async_trait]
impl crate::oidc::OpenIdConnect for Authentication {
    async fn start(
        &self,
        credentials: Credentials,
        request: StartRequest,
    ) -> Result<Authorization> {
        super::oidc::start(self.context.clone(), credentials, request).await
    }
    async fn complete(&self, credentials: Credentials, query: String) -> Result<OidcCompletion> {
        super::oidc::complete(&self.context, &credentials, &query).await
    }
}

#[async_trait]
impl crate::accounts::AccountAuthentication for Authentication {
    async fn bootstrap(&self) -> Result<BootstrapInfo> {
        super::accounts::bootstrap(self.context.clone()).await
    }
    async fn login(
        &self,
        credentials: Credentials,
        req: LoginRequest,
    ) -> Result<SessionUpdate<User>> {
        super::accounts::login(self.context.clone(), credentials, req).await
    }
    async fn logout(&self, credentials: Credentials) -> Result<SessionUpdate<MessageResponse>> {
        super::accounts::logout(self.context.clone(), credentials).await
    }
    async fn change_password(
        &self,
        credentials: Credentials,
        req: PasswordChange,
    ) -> Result<SessionUpdate<MessageResponse>> {
        super::accounts::change_password(self.context.clone(), credentials, req).await
    }
    async fn request_recovery(&self, req: RecoveryRequest) -> Result<MessageResponse> {
        super::accounts::request_recovery(self.context.clone(), req).await
    }
    async fn reset_password(&self, req: ResetRequest) -> Result<SessionUpdate<MessageResponse>> {
        super::accounts::reset_password(self.context.clone(), req).await
    }
    async fn sessions(&self, credentials: Credentials) -> Result<Vec<SessionInfo>> {
        super::accounts::sessions(self.context.clone(), credentials).await
    }
    async fn revoke_session(
        &self,
        credentials: Credentials,
        req: RevokeSession,
    ) -> Result<MessageResponse> {
        super::accounts::revoke_session(self.context.clone(), credentials, req).await
    }
    async fn admin_accounts(&self, credentials: Credentials) -> Result<Vec<AdminAccount>> {
        super::accounts::admin_accounts(self.context.clone(), credentials).await
    }
}
