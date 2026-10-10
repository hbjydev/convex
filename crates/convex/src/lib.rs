pub mod accounts;
pub mod config;
#[cfg(feature = "embed-web")]
mod frontend;
pub mod http;
pub mod security;
pub mod workspaces;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
