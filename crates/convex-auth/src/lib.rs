//! Accounts, credentials, sessions, authentication policy, and OpenID Connect.
//! HTTP cookie parsing and responses belong to convex-api.

#[cfg(feature = "postgres")]
pub mod postgres;
pub mod repository;
#[cfg(feature = "mock")]
pub mod mock {
    pub use crate::repository::{
        MockAccountRepository, MockAuthenticationPolicy, MockInstallationRepository,
        MockRateLimiter, MockSessionRepository,
    };
}

pub mod auth_policy;
pub mod credentials;
pub mod email;
pub mod methods;
pub mod methods_model;
pub mod oidc;
pub mod oidc_model;
pub mod security;

pub mod accounts;
