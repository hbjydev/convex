use crate::{
    http::{ApiJson, AppResult, AppState},
    security::credentials,
    workspaces,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Serialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub display_name: String,
    pub is_instance_admin: bool,
}

impl From<convex_auth::accounts::models::User> for User {
    fn from(value: convex_auth::accounts::models::User) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            is_instance_admin: value.is_instance_admin,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct BootstrapInfo {
    pub setup_required: bool,
    pub setup_enabled: bool,
    pub mail_available: bool,
    pub oidc_display_name: Option<String>,
    pub oidc_required: bool,
}

impl From<convex_auth::accounts::models::BootstrapInfo> for BootstrapInfo {
    fn from(value: convex_auth::accounts::models::BootstrapInfo) -> Self {
        Self {
            setup_required: value.setup_required,
            setup_enabled: value.setup_enabled,
            mail_available: value.mail_available,
            oidc_display_name: value.oidc_display_name,
            oidc_required: value.oidc_required,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetupRequest {
    pub setup_token: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub workspace_name: String,
    pub workspace_slug: String,
    pub team_name: String,
    pub team_key: String,
}

impl From<SetupRequest> for convex_app::accounts::models::SetupRequest {
    fn from(value: SetupRequest) -> Self {
        Self {
            setup_token: value.setup_token,
            email: value.email,
            password: value.password,
            display_name: value.display_name,
            workspace_name: value.workspace_name,
            workspace_slug: value.workspace_slug,
            team_name: value.team_name,
            team_key: value.team_key,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl From<LoginRequest> for convex_auth::accounts::models::LoginRequest {
    fn from(value: LoginRequest) -> Self {
        Self {
            email: value.email,
            password: value.password,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct AccountResponse {
    pub user: User,
    pub workspaces: Vec<workspaces::Workspace>,
}

impl From<convex_app::accounts::models::AccountResponse> for AccountResponse {
    fn from(value: convex_app::accounts::models::AccountResponse) -> Self {
        Self {
            user: value.user.into(),
            workspaces: value.workspaces.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

impl From<convex_auth::accounts::models::MessageResponse> for MessageResponse {
    fn from(value: convex_auth::accounts::models::MessageResponse) -> Self {
        Self {
            message: value.message,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PasswordChange {
    pub current_password: String,
    pub new_password: String,
}

impl From<PasswordChange> for convex_auth::accounts::models::PasswordChange {
    fn from(value: PasswordChange) -> Self {
        Self {
            current_password: value.current_password,
            new_password: value.new_password,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoveryRequest {
    pub email: String,
}

impl From<RecoveryRequest> for convex_auth::accounts::models::RecoveryRequest {
    fn from(value: RecoveryRequest) -> Self {
        Self { email: value.email }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResetRequest {
    pub token: String,
    pub password: String,
}

impl From<ResetRequest> for convex_auth::accounts::models::ResetRequest {
    fn from(value: ResetRequest) -> Self {
        Self {
            token: value.token,
            password: value.password,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct SessionInfo {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub current: bool,
}

impl From<convex_auth::accounts::models::SessionInfo> for SessionInfo {
    fn from(value: convex_auth::accounts::models::SessionInfo) -> Self {
        Self {
            id: value.id,
            created_at: value.created_at,
            expires_at: value.expires_at,
            last_activity_at: value.last_activity_at,
            current: value.current,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RevokeSession {
    pub session_id: Uuid,
}

impl From<RevokeSession> for convex_auth::accounts::models::RevokeSession {
    fn from(value: RevokeSession) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DisableRequest {
    pub user_id: Uuid,
    pub disabled: bool,
}

impl From<DisableRequest> for convex_app::accounts::models::DisableRequest {
    fn from(value: DisableRequest) -> Self {
        Self {
            user_id: value.user_id,
            disabled: value.disabled,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct AdminAccount {
    pub id: Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub disabled: bool,
    pub is_instance_admin: bool,
}

impl From<convex_auth::accounts::models::AdminAccount> for AdminAccount {
    fn from(value: convex_auth::accounts::models::AdminAccount) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
            email: value.email,
            disabled: value.disabled,
            is_instance_admin: value.is_instance_admin,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/bootstrap",
    operation_id = "getBootstrap",
    tag = "Accounts",
    responses(
        (status = 200, body = BootstrapInfo),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn bootstrap(State(state): State<AppState>) -> AppResult<Json<BootstrapInfo>> {
    let result = state.services.accounts.bootstrap().await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/setup",
    operation_id = "setupInstance",
    tag = "Accounts",
    request_body = SetupRequest,
    responses(
        (status = 200, body = User),
        (status = 400, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn setup(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<SetupRequest>,
) -> AppResult<Response> {
    let result = state.services.accounts.setup(req.into()).await?;
    Ok(session_response(&state, result, User::from))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    operation_id = "login",
    tag = "Accounts",
    request_body = LoginRequest,
    responses(
        (status = 200, body = User),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<LoginRequest>,
) -> AppResult<Response> {
    let result = state
        .services
        .accounts
        .login(credentials(&headers), req.into())
        .await?;
    Ok(session_response(&state, result, User::from))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    operation_id = "getAccount",
    tag = "Accounts",
    security(("cookieSession" = [])),
    responses(
        (status = 200, body = AccountResponse),
        (status = 401, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AccountResponse>> {
    let result = state.services.accounts.me(credentials(&headers)).await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    operation_id = "logout",
    tag = "Accounts",
    responses(
        (status = 200, body = MessageResponse),
        (status = 403, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let result = state
        .services
        .accounts
        .logout(credentials(&headers))
        .await?;
    Ok(session_response(&state, result, MessageResponse::from))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/password",
    operation_id = "changePassword",
    tag = "Accounts",
    security(("cookieSession" = [])),
    request_body = PasswordChange,
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<PasswordChange>,
) -> AppResult<Response> {
    let result = state
        .services
        .accounts
        .change_password(credentials(&headers), req.into())
        .await?;
    Ok(session_response(&state, result, MessageResponse::from))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/recovery",
    operation_id = "requestRecovery",
    tag = "Accounts",
    request_body = RecoveryRequest,
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn request_recovery(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<RecoveryRequest>,
) -> AppResult<Json<MessageResponse>> {
    let result = state.services.accounts.request_recovery(req.into()).await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/reset",
    operation_id = "resetPassword",
    tag = "Accounts",
    request_body = ResetRequest,
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn reset_password(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ResetRequest>,
) -> AppResult<Response> {
    let result = state.services.accounts.reset_password(req.into()).await?;
    Ok(session_response(&state, result, MessageResponse::from))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    operation_id = "listSessions",
    tag = "Accounts",
    security(("cookieSession" = [])),
    responses(
        (status = 200, body = Vec<SessionInfo>),
        (status = 401, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<SessionInfo>>> {
    let result = state
        .services
        .accounts
        .sessions(credentials(&headers))
        .await?;
    Ok(Json(result.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/sessions/revoke",
    operation_id = "revokeSession",
    tag = "Accounts",
    security(("cookieSession" = [])),
    request_body = RevokeSession,
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn revoke_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<RevokeSession>,
) -> AppResult<Json<MessageResponse>> {
    let result = state
        .services
        .accounts
        .revoke_session(credentials(&headers), req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/admin/accounts/disable",
    operation_id = "setAccountDisabled",
    tag = "Accounts",
    security(("cookieSession" = [])),
    request_body = DisableRequest,
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn disable(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<DisableRequest>,
) -> AppResult<Json<MessageResponse>> {
    let result = state
        .services
        .accounts
        .disable(credentials(&headers), req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/accounts",
    operation_id = "listAccounts",
    tag = "Accounts",
    security(("cookieSession" = [])),
    responses(
        (status = 200, body = Vec<AdminAccount>),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn admin_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AdminAccount>>> {
    let result = state
        .services
        .accounts
        .admin_accounts(credentials(&headers))
        .await?;
    Ok(Json(result.into_iter().map(Into::into).collect()))
}

pub(crate) fn session_response<T, D: Serialize>(
    state: &AppState,
    update: convex_auth::credentials::SessionUpdate<T>,
    convert: impl FnOnce(T) -> D,
) -> Response {
    let mut response = Json(convert(update.value)).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, state.security.session_cookie(&update.token));
    if update.clear_method_proof {
        response
            .headers_mut()
            .append(SET_COOKIE, state.security.proof_cookie(""));
    }
    response
}
