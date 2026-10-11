use crate::security::CookieConfig;
use convex_app::{
    IssueApplication,
    accounts::MockAccounts,
    mock::MockUnitOfWorkFactory,
    services::{MockReadiness, Services},
    workspaces::MockWorkspaces,
};
use convex_auth::{
    methods::MockLoginMethods, mock::MockAuthenticationPolicy, oidc::MockOpenIdConnect,
};
use std::sync::Arc;

pub fn services() -> Services {
    Services {
        accounts: Arc::new(MockAccounts::new()),
        workspaces: Arc::new(MockWorkspaces::new()),
        login_methods: Arc::new(MockLoginMethods::new()),
        oidc: Arc::new(MockOpenIdConnect::new()),
        issues: IssueApplication::new(Arc::new(MockUnitOfWorkFactory::new())),
        authentication_policy: Arc::new(MockAuthenticationPolicy::new()),
        readiness: Arc::new(MockReadiness::new()),
    }
}

pub fn cookies(origin: &str) -> CookieConfig {
    CookieConfig {
        origin: origin.into(),
        secure: false,
        session_lifetime_seconds: 604800,
    }
}

pub fn router() -> axum::Router {
    let mut dependencies = services();
    let mut readiness = MockReadiness::new();
    readiness
        .expect_check()
        .returning(|| Err(anyhow::anyhow!("Database unavailable")));
    dependencies.readiness = Arc::new(readiness);
    crate::router(cookies("http://127.0.0.1:5173"), dependencies)
}

#[cfg(feature = "embed-web")]
pub fn app(origin: &str) -> axum::Router {
    let mut dependencies = services();
    let mut readiness = MockReadiness::new();
    readiness
        .expect_check()
        .returning(|| Err(anyhow::anyhow!("Database unavailable")));
    dependencies.readiness = Arc::new(readiness);
    crate::app(cookies(origin), dependencies)
}
