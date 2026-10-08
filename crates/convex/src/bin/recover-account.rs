use anyhow::{Context, Result};
use convex::{
    MIGRATOR, accounts,
    config::Config,
    security::{self, SecurityConfig},
};
use uuid::Uuid;

/// Operator-only recovery: database access is the authorization boundary.
#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let email = security::email(
        &args
            .next()
            .context("Usage: recover-account EMAIL [--enable]")?,
    )?;
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
    let mut tx = accounts::transaction(&pool).await?;
    let (user,disabled):(Uuid,bool) = sqlx::query_as("SELECT u.id,u.disabled_at IS NOT NULL FROM convex.users u JOIN convex.local_credentials c ON c.user_id=u.id WHERE c.email=$1 FOR UPDATE OF u,c").bind(email).fetch_optional(&mut *tx).await?.context("No existing local credential for that email. OIDC-only accounts recover through their provider.")?;
    anyhow::ensure!(
        !disabled || enable,
        "Account is disabled. Add --enable to explicitly restore account access."
    );
    if enable {
        sqlx::query("UPDATE convex.users SET disabled_at=NULL,updated_at=now() WHERE id=$1")
            .bind(user)
            .execute(&mut *tx)
            .await?;
    }
    accounts::revoke_all(&mut tx, user).await?;
    let token = accounts::recovery_token(&mut tx, user).await?;
    tx.commit().await?;
    println!(
        "Single-use recovery link (expires in 30 minutes):\n{}/auth/reset#{}",
        security.origin, token
    );
    pool.close().await;
    Ok(())
}
