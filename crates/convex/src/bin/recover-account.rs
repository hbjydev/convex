use anyhow::{Context, Result};
use convex::{MIGRATOR, config::Config};
use convex_auth::security::SecurityConfig;

/// Operator-only recovery: database access is the authorization boundary.
#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let email = args
        .next()
        .context("Usage: recover-account EMAIL [--enable]")?;
    let enable = match args.next().as_deref() {
        None => false,
        Some("--enable") => true,
        _ => anyhow::bail!("Usage: recover-account EMAIL [--enable]"),
    };
    anyhow::ensure!(args.next().is_none(), "Unexpected argument");
    let config = Config::from_env()?;
    let security = SecurityConfig::from_env()?;
    let pool = config.connect().await?;
    MIGRATOR.run(&pool).await?;
    let token = convex_auth::postgres::accounts::recover_account(&pool, &email, enable).await?;
    println!(
        "Single-use recovery link (expires in 30 minutes):\n{}/auth/reset#{}",
        security.origin, token
    );
    pool.close().await;
    Ok(())
}
