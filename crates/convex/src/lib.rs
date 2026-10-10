pub mod accounts;
pub mod auth_policy;
pub mod config;
#[cfg(feature = "embed-web")]
mod frontend;
pub mod http;
pub mod issues;
pub mod methods;
pub mod oidc;
pub mod security;
pub mod workspaces;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
