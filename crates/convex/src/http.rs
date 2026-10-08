use std::collections::BTreeMap;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx::PgPool;
use tower_http::trace::TraceLayer;
use utoipa::{ToSchema, openapi::OpenApi};
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ServiceUnavailable,
    NotFound,
    MethodNotAllowed,
}

#[derive(Serialize, ToSchema)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_errors: Option<BTreeMap<String, Vec<String>>>,
}

impl ApiError {
    fn response(code: ErrorCode, message: &str, status: StatusCode) -> Response {
        (
            status,
            Json(Self {
                code,
                message: message.into(),
                field_errors: None,
            }),
        )
            .into_response()
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ready,
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
}

/// Reports whether the application can query the migrated PostgreSQL database.
#[utoipa::path(
    get, path = "/api/v1/health", operation_id = "getHealth", tag = "System",
    responses(
        (status = 200, description = "Application and database are ready", body = HealthResponse),
        (status = 503, description = "Database or application schema is unavailable", body = ApiError)
    )
)]
async fn health(State(pool): State<PgPool>) -> Response {
    let result = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'convex')",
    )
    .fetch_one(&pool)
    .await;
    match result {
        Ok(true) => Json(HealthResponse {
            status: HealthStatus::Ready,
            version: env!("CARGO_PKG_VERSION").into(),
        })
        .into_response(),
        result => {
            if let Err(error) = result {
                tracing::error!(%error, "Readiness query failed");
            }
            ApiError::response(
                ErrorCode::ServiceUnavailable,
                "The database is not ready. Try again shortly.",
                StatusCode::SERVICE_UNAVAILABLE,
            )
        }
    }
}

/// Returns the same contract exported by the offline generation command.
#[utoipa::path(
    get, path = "/api/openapi.json", operation_id = "getOpenApi", tag = "System",
    responses((status = 200, description = "OpenAPI 3.1 specification", body = Object))
)]
async fn specification() -> Json<OpenApi> {
    Json(openapi())
}

fn documented_router() -> OpenApiRouter<PgPool> {
    OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(specification))
}

pub fn openapi() -> OpenApi {
    let (_, mut document) = documented_router().split_for_parts();
    document.info = utoipa::openapi::InfoBuilder::new()
        .title("Convex API")
        .version(env!("CARGO_PKG_VERSION"))
        .description(Some("Public Convex HTTP contract. Unknown routes return 404 and unsupported methods return 405 using ApiError."))
        .build();
    document
}

pub fn router(pool: PgPool) -> Router {
    documented_router()
        .split_for_parts()
        .0
        .fallback(|| async {
            ApiError::response(
                ErrorCode::NotFound,
                "This endpoint does not exist.",
                StatusCode::NOT_FOUND,
            )
        })
        .method_not_allowed_fallback(|| async {
            ApiError::response(
                ErrorCode::MethodNotAllowed,
                "This endpoint does not support that method.",
                StatusCode::METHOD_NOT_ALLOWED,
            )
        })
        .layer(TraceLayer::new_for_http())
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn contract_and_errors_need_no_database() {
        let pool = PgPool::connect_lazy("postgres://unused:unused@127.0.0.1/unused").unwrap();
        pool.close().await;
        let app = router(pool);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let document: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(document, serde_json::to_value(openapi()).unwrap());
        assert_eq!(document["openapi"], "3.1.0");
        assert!(document["info"].get("contact").is_none());
        assert!(document["info"].get("license").is_none());
        assert_eq!(document["paths"].as_object().unwrap().len(), 2);
        assert_eq!(
            document["paths"]["/api/v1/health"]["get"]["operationId"],
            "getHealth"
        );

        for (method, uri, status, code) in [
            ("GET", "/api/v1/health", 503, "service_unavailable"),
            ("GET", "/missing", 404, "not_found"),
            ("POST", "/api/v1/health", 405, "method_not_allowed"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), status);
            assert_eq!(response.headers()["content-type"], "application/json");
            let body: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                    .unwrap();
            assert_eq!(body["code"], code);
            let schema = serde_json::json!({ "$ref": "#/components/schemas/ApiError", "components": document["components"] });
            jsonschema::validator_for(&schema)
                .unwrap()
                .validate(&body)
                .unwrap();
        }
    }
}
