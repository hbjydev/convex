//! Explicitly owned disposable databases for browser suites.
use super::Pool;
use anyhow::{Context, Result, ensure};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

pub struct TestDatabase {
    root: Pool,
    options: PgConnectOptions,
    name: String,
}

impl TestDatabase {
    pub async fn create(database_url: &str) -> Result<Self> {
        let options = database_url
            .parse::<PgConnectOptions>()?
            .options([("search_path", "public")]);
        ensure!(
            options
                .get_database()
                .is_some_and(|db| db.ends_with("_test")),
            "Browser tests require a test-database URL ending in _test"
        );
        let root = PgPoolOptions::new().connect_with(options.clone()).await?;
        let name = format!("convex_e2e_{}", uuid::Uuid::new_v4().simple());
        // The identifier is generated here, never supplied by configuration or a request.
        sqlx::query(&format!("CREATE DATABASE {name}"))
            .execute(&root)
            .await?;
        Ok(Self {
            root,
            options,
            name,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn connect(&self) -> Result<Pool> {
        PgPoolOptions::new()
            .connect_with(self.options.clone().database(&self.name))
            .await
            .context("could not connect to the browser-test database")
    }

    pub async fn close(self) -> Result<()> {
        sqlx::query(&format!("DROP DATABASE {} WITH (FORCE)", self.name))
            .execute(&self.root)
            .await?;
        self.root.close().await;
        Ok(())
    }
}
