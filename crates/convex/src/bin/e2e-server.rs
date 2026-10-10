//! Disposable database per browser-suite run; never clears the configured database.
use anyhow::{Context, Result};
use convex::{MIGRATOR, runtime};
use convex_auth::security::SecurityConfig;
use convex_db::postgres::testing::TestDatabase;

#[tokio::main]
async fn main() -> Result<()> {
    let database = TestDatabase::create(&std::env::var("DATABASE_URL")?).await?;
    // The test runner can invoke the real recovery command against this owned database.
    // Only its generated name is written here; database credentials stay in the environment.
    std::fs::create_dir_all("target")?;
    std::fs::write("target/e2e-database", database.name())?;
    let result = async {
        let pool = database.connect().await?;
        MIGRATOR.run(&pool).await?;
        let app = runtime::prepare_app(pool.clone(), SecurityConfig::from_env()?).await?;
        let listener = tokio::net::TcpListener::bind(
            std::env::var("CONVEX_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3001".into()),
        )
        .await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.expect("Ctrl-C handler");
            })
            .await
            .context("browser test API failed")?;
        pool.close().await;
        Ok::<_, anyhow::Error>(())
    }
    .await;
    database.close().await?;
    result
}
