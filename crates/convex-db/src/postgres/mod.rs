use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::{PgConnection, PgPool, Postgres, Transaction, postgres::PgPoolOptions};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

pub async fn connect(database_url: &str) -> Result<PgPool> {
    // Keep SQLx's migration ledger in public, regardless of the database role.
    // Domain queries explicitly qualify the convex schema.
    let options = database_url
        .parse::<sqlx::postgres::PgConnectOptions>()?
        .options([("search_path", "public")]);
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(options)
        .await
        .context("could not connect to PostgreSQL")
}

/// A single transaction, used as the PostgreSQL adapter for domain repositories.
/// Repository methods borrow its connection sequentially; only the application
/// unit of work commits it. Dropping an unfinished transaction rolls it back.
pub struct PostgresTransaction {
    transaction: Transaction<'static, Postgres>,
}

impl PostgresTransaction {
    pub async fn begin(pool: &PgPool) -> Result<Self> {
        Ok(Self {
            transaction: pool.begin().await?,
        })
    }

    /// Control-plane mutations acquire this gate before account and workspace locks.
    /// Issue operations instead use scoped workspace/issue locks.
    pub async fn begin_administration(pool: &PgPool) -> Result<Self> {
        let mut transaction = Self::begin(pool).await?;
        sqlx::query("SELECT singleton FROM convex.installation FOR UPDATE")
            .execute(transaction.connection())
            .await?;
        Ok(transaction)
    }

    pub fn transaction(&mut self) -> &mut Transaction<'static, Postgres> {
        &mut self.transaction
    }

    pub fn connection(&mut self) -> &mut PgConnection {
        &mut self.transaction
    }

    pub async fn commit(self) -> Result<()> {
        Ok(self.transaction.commit().await?)
    }

    pub async fn rollback(self) -> Result<()> {
        Ok(self.transaction.rollback().await?)
    }
}

pub type Pool = PgPool;

pub async fn schema_ready(pool: &Pool) -> Result<()> {
    let migrated: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'convex')")
            .fetch_one(pool)
            .await?;
    anyhow::ensure!(migrated, "Application schema has not been migrated");
    Ok(())
}

#[cfg(feature = "testing")]
pub mod testing;
