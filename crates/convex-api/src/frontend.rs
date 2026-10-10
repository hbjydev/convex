use axum::{
    Router,
    body::{Body, Bytes},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::Embed;
use tower_http::trace::TraceLayer;

#[derive(Embed)]
#[folder = "../../web/dist/"]
struct Frontend;

pub(crate) fn with_api(api: Router) -> Router {
    // route_service preserves the /api prefix expected by the existing router.
    // Its fallback and middleware remain scoped to API requests.
    Router::new()
        .route_service("/api", api.clone().into_service())
        .route_service("/api/", api.clone().into_service())
        .route_service("/api/{*path}", api.into_service())
        .fallback_service(get(serve).layer(TraceLayer::new_for_http()))
}

async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let (path, file) = match Frontend::get(path) {
        Some(file) => (path, file),
        // Missing files must not become successful HTML responses. All other
        // paths belong to the SPA, including its own unknown-page screen.
        None if path == "assets"
            || path.starts_with("assets/")
            || path
                .rsplit('/')
                .next()
                .is_some_and(|part| part.contains('.')) =>
        {
            return StatusCode::NOT_FOUND.into_response();
        }
        None => (
            "index.html",
            Frontend::get("index.html").expect("frontend build includes index.html"),
        ),
    };
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let bytes = match file.data {
        std::borrow::Cow::Borrowed(bytes) => Bytes::from_static(bytes),
        std::borrow::Cow::Owned(bytes) => Bytes::from(bytes),
    };
    Response::builder()
        .header(
            header::CONTENT_TYPE,
            mime_guess::from_path(path).first_or_octet_stream().as_ref(),
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .header(header::CACHE_CONTROL, cache)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(bytes))
        .expect("static frontend response headers")
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{body::to_bytes, http::Request};
    use tower::ServiceExt;

    async fn app() -> Router {
        crate::test_support::app("http://127.0.0.1:8080")
    }

    async fn request(app: &Router, method: &str, uri: &str) -> Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn frontend_routes_serve_the_embedded_index() {
        let app = app().await;
        let index = Frontend::get("index.html").unwrap();
        for path in [
            "/",
            "/index.html",
            "/account",
            "/auth/login",
            "/workspaces/example",
            "/unknown-page?from=test",
        ] {
            let response = request(&app, "GET", path).await;
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_eq!(response.headers()[header::CONTENT_TYPE], "text/html");
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
            assert_eq!(
                response.headers()[header::X_CONTENT_TYPE_OPTIONS],
                "nosniff"
            );
            assert_eq!(
                to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .as_ref(),
                index.data.as_ref()
            );
        }
        let response = request(&app, "HEAD", "/account").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_LENGTH],
            index.data.len().to_string()
        );
        assert!(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn built_assets_have_correct_types_and_cache_headers() {
        let app = app().await;
        for (extension, content_type) in [
            (".js", "text/javascript"),
            (".css", "text/css"),
            (".woff2", "font/woff2"),
        ] {
            let path = Frontend::iter()
                .find(|path| path.ends_with(extension))
                .expect("Vite asset exists");
            let response = request(&app, "GET", &format!("/{path}")).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CONTENT_TYPE], content_type);
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "public, max-age=31536000, immutable"
            );
            assert_eq!(
                to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .as_ref(),
                Frontend::get(&path).unwrap().data.as_ref()
            );
        }
        for path in [
            "/assets",
            "/assets/missing.js",
            "/assets/missing",
            "/favicon.ico",
        ] {
            assert_eq!(
                request(&app, "GET", path).await.status(),
                StatusCode::NOT_FOUND
            );
        }
        assert_eq!(
            request(&app, "POST", "/account").await.status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[tokio::test]
    async fn api_errors_contract_and_csrf_are_preserved() {
        let app = app().await;
        for path in ["/api", "/api/", "/api/missing", "/api/v1/missing"] {
            let response = request(&app, "GET", path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let body: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                    .unwrap();
            assert_eq!(body["code"], "not_found");
        }
        let response = request(&app, "GET", "/api/openapi.json").await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body, serde_json::to_value(crate::openapi()).unwrap());
        assert_eq!(
            request(&app, "GET", "/api/v1/health").await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            request(&app, "POST", "/api/v1/health").await.status(),
            StatusCode::FORBIDDEN
        );
        for (origin, expected) in [
            ("http://127.0.0.1:5173", StatusCode::FORBIDDEN),
            ("http://127.0.0.1:8080", StatusCode::METHOD_NOT_ALLOWED),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/health")
                        .header("origin", origin)
                        .header("x-convex-csrf", "1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
        }
    }
}
