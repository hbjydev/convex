use std::{env, net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

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

    pub async fn connect(&self) -> Result<PgPool> {
        // Keep SQLx's migration ledger in public even when the database role is
        // named convex. Domain queries will explicitly qualify the convex schema.
        let options = self
            .database_url
            .parse::<PgConnectOptions>()?
            .options([("search_path", "public")]);
        PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(3))
            .connect_with(options)
            .await
            .context("could not connect to PostgreSQL")
    }
}
