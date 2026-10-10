use std::collections::BTreeMap;

use crate::{accounts, issues, methods, oidc, security::CookieConfig, workspaces};
use axum::{
    Json, Router,
    extract::{FromRequest, Request, State},
    http::HeaderValue,
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use convex_app::services::Services;
use serde::{Serialize, de::DeserializeOwned};
use tower_http::trace::TraceLayer;
use utoipa::{ToSchema, openapi::OpenApi};
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Validation,
    Unauthorized,
    Forbidden,
    Conflict,
    RateLimited,
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
    pub fn response(code: ErrorCode, message: &str, status: StatusCode) -> Response {
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
    get,
    path = "/api/v1/health",
    operation_id = "getHealth",
    tag = "System",
    responses(
        (status = 200, description = "Application and database are ready", body = HealthResponse),
        (
            status = 503,
            description = "Database, application schema, or authentication policy lease is unavailable",
            body = ApiError,
        ),
    ),
)]
async fn health(State(state): State<AppState>) -> Response {
    match state.services.readiness.check().await {
        Ok(()) => Json(HealthResponse {
            status: HealthStatus::Ready,
            version: env!("CARGO_PKG_VERSION").into(),
        })
        .into_response(),
        Err(error) => {
            tracing::error!(%error, "Readiness check failed");
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
    get,
    path = "/api/openapi.json",
    operation_id = "getOpenApi",
    tag = "System",
    responses(
        (status = 200, description = "OpenAPI 3.1 specification", body = Object),
    ),
)]
async fn specification() -> Json<OpenApi> {
    Json(openapi())
}

fn documented_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(specification))
        .routes(routes!(accounts::bootstrap))
        .routes(routes!(oidc::start))
        .routes(routes!(oidc::callback))
        .routes(routes!(methods::list))
        .routes(routes!(methods::link))
        .routes(routes!(methods::prove))
        .routes(routes!(methods::add_local))
        .routes(routes!(methods::remove_local))
        .routes(routes!(methods::remove_oidc))
        .routes(routes!(accounts::setup))
        .routes(routes!(accounts::login))
        .routes(routes!(accounts::me))
        .routes(routes!(accounts::logout))
        .routes(routes!(accounts::change_password))
        .routes(routes!(accounts::request_recovery))
        .routes(routes!(accounts::reset_password))
        .routes(routes!(accounts::sessions))
        .routes(routes!(accounts::revoke_session))
        .routes(routes!(accounts::disable))
        .routes(routes!(accounts::admin_accounts))
        .routes(routes!(issues::create, issues::list))
        .routes(routes!(issues::detail, issues::update))
        .routes(routes!(workspaces::create))
        .routes(routes!(workspaces::detail))
        .routes(routes!(workspaces::rename))
        .routes(routes!(workspaces::team))
        .routes(routes!(workspaces::rename_team))
        .routes(routes!(workspaces::change_member))
        .routes(routes!(workspaces::invite))
        .routes(routes!(workspaces::invitations))
        .routes(routes!(workspaces::revoke_invitation))
        .routes(routes!(workspaces::preview))
        .routes(routes!(workspaces::accept))
}

pub fn openapi() -> OpenApi {
    let (_, mut document) = documented_router().split_for_parts();
    document.info = utoipa::openapi::InfoBuilder::new()
        .title("Convex API")
        .version(env!("CARGO_PKG_VERSION"))
        .description(Some("Public Convex HTTP contract. Unknown routes return 404 and unsupported methods return 405 using ApiError."))
        .build();
    document
        .components
        .as_mut()
        .expect("schemas registered")
        .add_security_scheme(
            "cookieSession",
            utoipa::openapi::security::SecurityScheme::ApiKey(
                utoipa::openapi::security::ApiKey::Cookie(
                    utoipa::openapi::security::ApiKeyValue::new("convex_session"),
                ),
            ),
        );
    document.info.description = Some("Convex HTTP contract. Mutations require Origin matching CONVEX_PUBLIC_URL and X-Convex-CSRF: 1, including public authentication endpoints. Cookies are HttpOnly, SameSite=Lax, Secure on HTTPS, and use CONVEX_SESSION_TTL_SECONDS (default seven days), with no sliding renewal. Shortening the lifetime caps existing sessions; increasing it does not extend them. Tokens in invitation/reset links use URL fragments and are submitted in JSON bodies. Errors use ApiError. Protected operations recheck active accounts, workspace membership, and authentication policy without contacting an identity provider. Logout revokes only the Convex session.".into());
    document
}

/// Builds the API from dependencies configured by the executable.
pub fn router(security: CookieConfig, services: Services) -> Router {
    let state = AppState { security, services };
    documented_router()
        .split_for_parts()
        .0
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            authentication_policy,
        ))
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
        .layer(TraceLayer::new_for_http().make_span_with(|request: &Request| {
            tracing::info_span!("http_request", method = %request.method(), path = %request.uri().path())
        }))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            csrf_and_cache,
        ))
        .with_state(state)
}

/// Serves the API and, when enabled, the frontend on the same origin.
pub fn app(security: CookieConfig, services: Services) -> Router {
    let api = router(security, services);
    #[cfg(feature = "embed-web")]
    {
        crate::frontend::with_api(api)
    }
    #[cfg(not(feature = "embed-web"))]
    {
        api
    }
}

#[derive(Clone)]
pub struct AppState {
    pub security: CookieConfig,
    pub services: Services,
}
impl AppState {
    async fn check_policy(&self) -> AppResult<()> {
        self.services
            .authentication_policy
            .check()
            .await
            .map_err(AppError::internal)
    }
}

pub type AppResult<T> = Result<T, AppError>;
#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub code: ErrorCode,
    pub message: String,
}
impl AppError {
    fn new(status: StatusCode, code: ErrorCode, message: &str) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
    pub fn validation(message: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorCode::Validation, message)
    }
    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthorized,
            "Sign in with valid credentials to continue.",
        )
    }
    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            ErrorCode::Forbidden,
            "You do not have permission to perform this action.",
        )
    }
    pub fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            ErrorCode::NotFound,
            "This resource does not exist or is not available to you.",
        )
    }
    pub fn conflict(message: &str) -> Self {
        Self::new(StatusCode::CONFLICT, ErrorCode::Conflict, message)
    }
    pub fn rate_limited() -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::RateLimited,
            "Too many attempts. Wait 15 minutes, then try again.",
        )
    }
    pub fn internal(error: impl std::fmt::Display) -> Self {
        tracing::error!(%error,"Operation failed");
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ServiceUnavailable,
            "The service is temporarily unavailable. Try again shortly.",
        )
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for AppError {}
impl From<convex_core::Error> for AppError {
    fn from(error: convex_core::Error) -> Self {
        use convex_core::Error;
        match error {
            Error::Validation(message) => Self::validation(&message),
            Error::Unauthorized => Self::unauthorized(),
            Error::Forbidden => Self::forbidden(),
            Error::NotFound => Self::not_found(),
            Error::Conflict(message) => Self::conflict(&message),
            Error::RateLimited => Self::rate_limited(),
            Error::Infrastructure(error) => Self::internal(error),
        }
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        ApiError::response(self.code, &self.message, self.status)
    }
}

pub struct ApiJson<T>(pub T);
impl<T: DeserializeOwned + Send, S: Send + Sync> FromRequest<S> for ApiJson<T> {
    type Rejection = AppError;
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(req, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|_| AppError::validation("Send a valid JSON body with the required fields."))
    }
}
async fn csrf_and_cache(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let method = request.method();
    let mut response = if !matches!(
        *method,
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    ) && (request
        .headers()
        .get("x-convex-csrf")
        .and_then(|v| v.to_str().ok())
        != Some("1")
        || request
            .headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
            != Some(state.security.origin.as_str()))
    {
        AppError::forbidden().into_response()
    } else {
        next.run(request).await
    };
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn authentication_policy(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // route_layer applies only to registered methods, preserving database-free
    // unknown-route/method errors as well as health and contract requests.
    if !matches!(request.uri().path(), "/api/openapi.json" | "/api/v1/health")
        && let Err(error) = state.check_policy().await
    {
        return error.into_response();
    }
    next.run(request).await
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
        let app = crate::test_support::router();
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
        assert!(document["paths"].as_object().unwrap().len() > 20);
        assert_eq!(
            document["paths"]["/api/v1/health"]["get"]["operationId"],
            "getHealth"
        );

        for (method, uri, status, code) in [
            ("GET", "/api/v1/health", 503, "service_unavailable"),
            ("GET", "/missing", 404, "not_found"),
            ("GET", "/api/v1/auth/missing", 404, "not_found"),
            (
                "GET",
                "/api/v1/workspaces/missing/unknown",
                404,
                "not_found",
            ),
            ("POST", "/api/v1/health", 405, "method_not_allowed"),
            ("POST", "/api/v1/auth/me", 405, "method_not_allowed"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .header("origin", "http://127.0.0.1:5173")
                        .header("x-convex-csrf", "1")
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
