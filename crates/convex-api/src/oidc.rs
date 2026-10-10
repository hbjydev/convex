use crate::{
    http::{ApiJson, AppResult, AppState},
    security::credentials,
};
use axum::{
    Json,
    extract::{RawQuery, State},
    http::{HeaderMap, HeaderValue, header::SET_COOKIE},
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartRequest {
    pub invitation_token: Option<String>,
}

impl From<StartRequest> for convex_auth::oidc_model::StartRequest {
    fn from(value: StartRequest) -> Self {
        Self {
            invitation_token: value.invitation_token,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct StartResponse {
    pub authorization_url: String,
}

impl From<convex_auth::oidc_model::StartResponse> for StartResponse {
    fn from(value: convex_auth::oidc_model::StartResponse) -> Self {
        Self {
            authorization_url: value.authorization_url,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/oidc/start",
    operation_id = "startOidcLogin",
    tag = "Accounts",
    request_body = StartRequest,
    responses(
        (
            status = 200,
            body = StartResponse,
            description = "Authorization URL and HttpOnly browser-binding cookie; requires same-origin CSRF headers",
        ),
        (status = 400, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn start(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<StartRequest>,
) -> AppResult<Response> {
    let result = state
        .services
        .oidc
        .start(credentials(&headers), req.into())
        .await?;
    authorization(&state, result)
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/oidc/callback",
    operation_id = "completeOidcLogin",
    tag = "Accounts",
    params(
        (
            "state" = String,
            Query,
            description = "Provider-returned opaque state; requires matching browser-binding cookie",
        ),
        (
            "code" = Option<String>,
            Query,
            description = "Authorization code; exactly one of code or error",
        ),
        (
            "error" = Option<String>,
            Query,
            description = "Provider cancellation or denial; details are discarded",
        ),
    ),
    responses(
        (
            status = 303,
            description = "Single-use browser-bound callback. Redirects to fixed login outcome or account method destination; session cookie for successful login/linking, HttpOnly proof cookie for reauthentication. Cache-Control: no-store; Referrer-Policy: no-referrer. Provider errors are sanitized.",
        ),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> Response {
    let result = state
        .services
        .oidc
        .complete(credentials(&headers), raw.unwrap_or_default())
        .await;
    let (destination, session, proof) = match result {
        Ok(convex_auth::credentials::OidcCompletion::Login(token)) => {
            ("/auth/oidc?outcome=success", Some(token), None)
        }
        Ok(convex_auth::credentials::OidcCompletion::Linked(token)) => {
            ("/account?method=linked", Some(token), None)
        }
        Ok(convex_auth::credentials::OidcCompletion::Proof(token)) => {
            ("/account?method=proof", None, Some(token))
        }
        Err(_) => ("/auth/oidc?outcome=failed", None, None),
    };
    let mut response =
        Redirect::to(&format!("{}{destination}", state.security.origin)).into_response();
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    if let Some(token) = session {
        response
            .headers_mut()
            .append(SET_COOKIE, state.security.session_cookie(&token));
        response
            .headers_mut()
            .append(SET_COOKIE, state.security.proof_cookie(""));
    }
    if let Some(token) = proof {
        response
            .headers_mut()
            .append(SET_COOKIE, state.security.proof_cookie(&token));
    }
    response
}
pub(crate) fn authorization(
    state: &AppState,
    result: convex_auth::credentials::Authorization,
) -> AppResult<Response> {
    let mut response = Json(StartResponse {
        authorization_url: result.authorization_url,
    })
    .into_response();
    response.headers_mut().append(
        SET_COOKIE,
        state.security.browser_cookie(&result.browser_token),
    );
    Ok(response)
}
