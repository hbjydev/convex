use crate::credentials::*;
use crate::{accounts::models::MessageResponse, methods_model::*};
use async_trait::async_trait;
use convex_core::Result;

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait LoginMethods: Send + Sync {
    async fn list(&self, credentials: Credentials) -> Result<Methods>;
    async fn link(&self, credentials: Credentials, req: LocalProof) -> Result<Authorization>;
    async fn prove(&self, credentials: Credentials, req: ProveOidc) -> Result<Authorization>;
    async fn add_local(
        &self,
        credentials: Credentials,
        req: AddLocal,
    ) -> Result<SessionUpdate<MessageResponse>>;
    async fn remove_local(
        &self,
        credentials: Credentials,
    ) -> Result<SessionUpdate<MessageResponse>>;
    async fn remove_oidc(
        &self,
        credentials: Credentials,
        req: RemoveOidc,
    ) -> Result<SessionUpdate<MessageResponse>>;
}
