use crate::{
    accounts,
    http::{ApiJson, AppResult, AppState},
    oidc,
    security::credentials,
};
use axum::{Json, extract::State, http::HeaderMap, response::Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MethodAction {
    AddLocal,
    RemoveLocal,
}

impl From<MethodAction> for convex_auth::methods_model::MethodAction {
    fn from(value: MethodAction) -> Self {
        match value {
            MethodAction::AddLocal => Self::AddLocal,
            MethodAction::RemoveLocal => Self::RemoveLocal,
        }
    }
}
impl From<convex_auth::methods_model::MethodAction> for MethodAction {
    fn from(value: convex_auth::methods_model::MethodAction) -> Self {
        match value {
            convex_auth::methods_model::MethodAction::AddLocal => Self::AddLocal,
            convex_auth::methods_model::MethodAction::RemoveLocal => Self::RemoveLocal,
        }
    }
}
#[derive(Serialize, ToSchema)]
pub struct OidcMethod {
    pub id: Uuid,
    pub issuer: String,
    pub usable: bool,
}

impl From<convex_auth::methods_model::OidcMethod> for OidcMethod {
    fn from(value: convex_auth::methods_model::OidcMethod) -> Self {
        Self {
            id: value.id,
            issuer: value.issuer,
            usable: value.usable,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct Methods {
    pub local_email: Option<String>,
    pub local_usable: bool,
    pub oidc: Vec<OidcMethod>,
    pub provider_display_name: Option<String>,
    pub can_link_oidc: bool,
    pub can_add_local: bool,
    pub can_remove_local: bool,
    pub pending_action: Option<MethodAction>,
}

impl From<convex_auth::methods_model::Methods> for Methods {
    fn from(value: convex_auth::methods_model::Methods) -> Self {
        Self {
            local_email: value.local_email,
            local_usable: value.local_usable,
            oidc: value.oidc.into_iter().map(Into::into).collect(),
            provider_display_name: value.provider_display_name,
            can_link_oidc: value.can_link_oidc,
            can_add_local: value.can_add_local,
            can_remove_local: value.can_remove_local,
            pending_action: value.pending_action.map(Into::into),
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalProof {
    pub current_password: String,
}

impl From<LocalProof> for convex_auth::methods_model::LocalProof {
    fn from(value: LocalProof) -> Self {
        Self {
            current_password: value.current_password,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProveOidc {
    pub action: MethodAction,
}

impl From<ProveOidc> for convex_auth::methods_model::ProveOidc {
    fn from(value: ProveOidc) -> Self {
        Self {
            action: value.action.into(),
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddLocal {
    pub email: String,
    pub password: String,
}

impl From<AddLocal> for convex_auth::methods_model::AddLocal {
    fn from(value: AddLocal) -> Self {
        Self {
            email: value.email,
            password: value.password,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveOidc {
    pub identity_id: Uuid,
    pub current_password: String,
}

impl From<RemoveOidc> for convex_auth::methods_model::RemoveOidc {
    fn from(value: RemoveOidc) -> Self {
        Self {
            identity_id: value.identity_id,
            current_password: value.current_password,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/methods",
    operation_id = "getLoginMethods",
    tag = "Accounts",
    security(("cookieSession" = [])),
    responses(
        (status = 200, body = Methods),
        (status = 401, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn list(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Methods>> {
    let result = state
        .services
        .login_methods
        .list(credentials(&headers))
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/methods/oidc/link",
    operation_id = "linkOidcMethod",
    tag = "Accounts",
    security(("cookieSession" = [])),
    request_body = LocalProof,
    responses(
        (status = 200, body = oidc::StartResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn link(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<LocalProof>,
) -> AppResult<Response> {
    let result = state
        .services
        .login_methods
        .link(credentials(&headers), req.into())
        .await?;
    oidc::authorization(&state, result)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/methods/oidc/prove",
    operation_id = "proveOidcMethod",
    tag = "Accounts",
    security(("cookieSession" = [])),
    request_body = ProveOidc,
    responses(
        (status = 200, body = oidc::StartResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn prove(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<ProveOidc>,
) -> AppResult<Response> {
    let result = state
        .services
        .login_methods
        .prove(credentials(&headers), req.into())
        .await?;
    oidc::authorization(&state, result)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/methods/local/add",
    operation_id = "addLocalMethod",
    tag = "Accounts",
    security(("cookieSession" = [])),
    request_body = AddLocal,
    responses(
        (status = 200, body = accounts::MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn add_local(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<AddLocal>,
) -> AppResult<Response> {
    let result = state
        .services
        .login_methods
        .add_local(credentials(&headers), req.into())
        .await?;
    Ok(accounts::session_response(
        &state,
        result,
        accounts::MessageResponse::from,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/methods/local/remove",
    operation_id = "removeLocalMethod",
    tag = "Accounts",
    security(("cookieSession" = [])),
    responses(
        (status = 200, body = accounts::MessageResponse),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn remove_local(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let result = state
        .services
        .login_methods
        .remove_local(credentials(&headers))
        .await?;
    Ok(accounts::session_response(
        &state,
        result,
        accounts::MessageResponse::from,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/methods/oidc/remove",
    operation_id = "removeOidcMethod",
    tag = "Accounts",
    security(("cookieSession" = [])),
    request_body = RemoveOidc,
    responses(
        (status = 200, body = accounts::MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn remove_oidc(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<RemoveOidc>,
) -> AppResult<Response> {
    let result = state
        .services
        .login_methods
        .remove_oidc(credentials(&headers), req.into())
        .await?;
    Ok(accounts::session_response(
        &state,
        result,
        accounts::MessageResponse::from,
    ))
}
