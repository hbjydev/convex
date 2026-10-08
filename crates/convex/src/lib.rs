pub mod accounts;
pub mod config;
pub mod http;
pub mod security;
pub mod workspaces;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
