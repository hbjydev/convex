use axum::Router;
use convex_app::postgres::ApplicationBackend;
use convex_auth::security::SecurityConfig;
use convex_db::postgres::Pool;

pub fn router(pool: Pool) -> Router {
    router_with_config(pool, SecurityConfig::development())
}

pub fn router_with_config(pool: Pool, configuration: SecurityConfig) -> Router {
    let cookies = (&configuration).into();
    let services = ApplicationBackend::new(pool, configuration).services();
    convex_api::router(cookies, services)
}

pub fn app_with_config(pool: Pool, configuration: SecurityConfig) -> Router {
    let cookies = (&configuration).into();
    let services = ApplicationBackend::new(pool, configuration).services();
    convex_api::app(cookies, services)
}

/// Activate the replica's policy lease before accepting requests.
pub async fn prepare_app(pool: Pool, configuration: SecurityConfig) -> anyhow::Result<Router> {
    let cookies = (&configuration).into();
    let backend = ApplicationBackend::new(pool, configuration);
    backend.prepare().await?;
    Ok(convex_api::app(cookies, backend.services()))
}
