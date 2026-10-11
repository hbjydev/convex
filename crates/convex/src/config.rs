use std::{env, net::SocketAddr};

use anyhow::{Context, Result};
use convex_db::postgres::Pool;

pub struct Config {
    pub bind_address: SocketAddr,
    database_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            bind_address: env::var("CONVEX_BIND_ADDRESS")
                .unwrap_or_else(|_| "127.0.0.1:3000".into())
                .parse()
                .context("CONVEX_BIND_ADDRESS must be an IP address and port")?,
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is required")?,
        })
    }

    pub async fn connect(&self) -> Result<Pool> {
        convex_db::postgres::connect(&self.database_url).await
    }
}
