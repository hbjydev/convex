//! Disposable database per browser-suite run; never clears the configured database.
use anyhow::{Context, Result};
use convex::{MIGRATOR, http, security::SecurityConfig};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[tokio::main]
async fn main() -> Result<()> {
    let options = std::env::var("DATABASE_URL")?
        .parse::<PgConnectOptions>()?
        .options([("search_path", "public")]);
    anyhow::ensure!(
        options
            .get_database()
            .is_some_and(|db| db.ends_with("_test")),
        "Browser tests require a test-database URL ending in _test"
    );
    let root = PgPoolOptions::new().connect_with(options.clone()).await?;
    let database = format!("convex_e2e_{}", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(&root)
        .await?;
    // The test runner can invoke the real recovery command against this owned database.
    // Only its generated name is written here; database credentials stay in the environment.
    std::fs::create_dir_all("target")?;
    std::fs::write("target/e2e-database", &database)?;
    let result = async {
        let pool = PgPoolOptions::new()
            .connect_with(options.database(&database))
            .await?;
        MIGRATOR.run(&pool).await?;
        let app = http::prepare_app(pool.clone(), SecurityConfig::from_env()?).await?;
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
    sqlx::query(&format!("DROP DATABASE {database} WITH (FORCE)"))
        .execute(&root)
        .await?;
    root.close().await;
    result
}
