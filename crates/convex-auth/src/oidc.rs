use crate::credentials::*;
use crate::oidc_model::StartRequest;
use async_trait::async_trait;
use convex_core::Result;

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait OpenIdConnect: Send + Sync {
    async fn start(&self, credentials: Credentials, request: StartRequest)
    -> Result<Authorization>;
    async fn complete(&self, credentials: Credentials, query: String) -> Result<OidcCompletion>;
}
