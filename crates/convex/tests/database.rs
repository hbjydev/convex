use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use convex::{MIGRATOR, http};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn empty_database_migrates_and_responds_with_documented_health(
    pool_options: PgPoolOptions,
    connect_options: PgConnectOptions,
) {
    let pool = pool_options
        .connect_with(connect_options.options([("search_path", "public")]))
        .await
        .unwrap();
    // Reapplying migrations must be harmless and preserve the migration ledger.
    MIGRATOR.run(&pool).await.unwrap();
    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(applied, 1);

    let response = http::router(pool)
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let contract = serde_json::to_value(http::openapi()).unwrap();
    let schema = json!({ "$ref": "#/components/schemas/HealthResponse", "components": contract["components"] });
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&body)
        .unwrap();
    assert_eq!(body["status"], "ready");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[sqlx::test(migrations = false)]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn reachable_but_unmigrated_database_is_not_ready(pool: PgPool) {
    let response = http::router(pool)
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
