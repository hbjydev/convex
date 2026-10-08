#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = convex::config::Config::from_env()?.connect().await?;
    convex::MIGRATOR.run(&pool).await?;
    pool.close().await;
    Ok(())
}
