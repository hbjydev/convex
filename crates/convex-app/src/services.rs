use crate::{IssueApplication, accounts::Accounts, workspaces::Workspaces};
use async_trait::async_trait;
use convex_auth::{methods::LoginMethods, oidc::OpenIdConnect, repository::AuthenticationPolicy};
use std::sync::Arc;

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait Readiness: Send + Sync {
    async fn check(&self) -> anyhow::Result<()>;
}

/// Every dependency consumed by an HTTP adapter; all are transport-independent.
#[derive(Clone)]
pub struct Services {
    pub accounts: Arc<dyn Accounts>,
    pub workspaces: Arc<dyn Workspaces>,
    pub login_methods: Arc<dyn LoginMethods>,
    pub oidc: Arc<dyn OpenIdConnect>,
    pub issues: IssueApplication,
    pub authentication_policy: Arc<dyn AuthenticationPolicy>,
    pub readiness: Arc<dyn Readiness>,
}
