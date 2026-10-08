pub mod config;
pub mod http;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
