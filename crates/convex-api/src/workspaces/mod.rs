use crate::{
    accounts::{self, MessageResponse},
    http::{ApiJson, AppResult, AppState},
    security::credentials,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: String,
}

impl From<convex_workspaces::workspaces_model::Workspace> for Workspace {
    fn from(value: convex_workspaces::workspaces_model::Workspace) -> Self {
        Self {
            id: value.id,
            name: value.name,
            slug: value.slug,
            role: value.role,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct Member {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub role: String,
    pub disabled: bool,
}

impl From<convex_workspaces::workspaces_model::Member> for Member {
    fn from(value: convex_workspaces::workspaces_model::Member) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            display_name: value.display_name,
            role: value.role,
            disabled: value.disabled,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct WorkflowStatus {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub position: i32,
}

impl From<convex_workspaces::workspaces_model::WorkflowStatus> for WorkflowStatus {
    fn from(value: convex_workspaces::workspaces_model::WorkflowStatus) -> Self {
        Self {
            id: value.id,
            name: value.name,
            category: value.category,
            position: value.position,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub key: String,
    pub statuses: Vec<WorkflowStatus>,
}

impl From<convex_workspaces::workspaces_model::Team> for Team {
    fn from(value: convex_workspaces::workspaces_model::Team) -> Self {
        Self {
            id: value.id,
            name: value.name,
            key: value.key,
            statuses: value.statuses.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct WorkspaceDetail {
    pub workspace: Workspace,
    pub members: Vec<Member>,
    pub teams: Vec<Team>,
}

impl From<convex_workspaces::workspaces_model::WorkspaceDetail> for WorkspaceDetail {
    fn from(value: convex_workspaces::workspaces_model::WorkspaceDetail) -> Self {
        Self {
            workspace: value.workspace.into(),
            members: value.members.into_iter().map(Into::into).collect(),
            teams: value.teams.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateWorkspace {
    pub name: String,
    pub slug: String,
    pub team_name: String,
    pub team_key: String,
}

impl From<CreateWorkspace> for convex_workspaces::workspaces_model::CreateWorkspace {
    fn from(value: CreateWorkspace) -> Self {
        Self {
            name: value.name,
            slug: value.slug,
            team_name: value.team_name,
            team_key: value.team_key,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TeamRequest {
    pub name: String,
    pub key: String,
}

impl From<TeamRequest> for convex_workspaces::workspaces_model::TeamRequest {
    fn from(value: TeamRequest) -> Self {
        Self {
            name: value.name,
            key: value.key,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameRequest {
    pub name: String,
}

impl From<RenameRequest> for convex_workspaces::workspaces_model::RenameRequest {
    fn from(value: RenameRequest) -> Self {
        Self { name: value.name }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MembershipChange {
    pub membership_id: Uuid,
    pub role: Option<String>,
    pub remove: bool,
}

impl From<MembershipChange> for convex_workspaces::workspaces_model::MembershipChange {
    fn from(value: MembershipChange) -> Self {
        Self {
            membership_id: value.membership_id,
            role: value.role,
            remove: value.remove,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InviteRequest {
    pub role: String,
    pub deliver_to: Option<String>,
}

impl From<InviteRequest> for convex_workspaces::workspaces_model::InviteRequest {
    fn from(value: InviteRequest) -> Self {
        Self {
            role: value.role,
            deliver_to: value.deliver_to,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct InviteLink {
    pub id: Uuid,
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub email_delivered: bool,
}

impl From<convex_workspaces::workspaces_model::InviteLink> for InviteLink {
    fn from(value: convex_workspaces::workspaces_model::InviteLink) -> Self {
        Self {
            id: value.id,
            url: value.url,
            expires_at: value.expires_at,
            email_delivered: value.email_delivered,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct Invitation {
    pub id: Uuid,
    pub role: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl From<convex_workspaces::workspaces_model::Invitation> for Invitation {
    fn from(value: convex_workspaces::workspaces_model::Invitation) -> Self {
        Self {
            id: value.id,
            role: value.role,
            expires_at: value.expires_at,
            accepted_at: value.accepted_at,
            revoked_at: value.revoked_at,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RevokeInvitation {
    pub invitation_id: Uuid,
}

impl From<RevokeInvitation> for convex_workspaces::workspaces_model::RevokeInvitation {
    fn from(value: RevokeInvitation) -> Self {
        Self {
            invitation_id: value.invitation_id,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InviteToken {
    pub token: String,
}

impl From<InviteToken> for convex_workspaces::workspaces_model::InviteToken {
    fn from(value: InviteToken) -> Self {
        Self { token: value.token }
    }
}

#[derive(Serialize, ToSchema)]
pub struct InvitePreview {
    pub workspace_name: String,
    pub role: String,
}

impl From<convex_workspaces::workspaces_model::InvitePreview> for InvitePreview {
    fn from(value: convex_workspaces::workspaces_model::InvitePreview) -> Self {
        Self {
            workspace_name: value.workspace_name,
            role: value.role,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptInvite {
    pub token: String,
    pub email: Option<String>,
    pub password: Option<String>,
    pub display_name: Option<String>,
}

impl From<AcceptInvite> for convex_app::accounts::models::AcceptInvite {
    fn from(value: AcceptInvite) -> Self {
        Self {
            token: value.token,
            email: value.email,
            password: value.password,
            display_name: value.display_name,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces",
    operation_id = "createWorkspace",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    request_body = CreateWorkspace,
    responses(
        (status = 200, body = Workspace),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<CreateWorkspace>,
) -> AppResult<Json<Workspace>> {
    let result = state
        .services
        .workspaces
        .create(credentials(&headers), req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}",
    operation_id = "getWorkspace",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
    ),
    responses(
        (status = 200, body = WorkspaceDetail),
        (status = 401, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
) -> AppResult<Json<WorkspaceDetail>> {
    let result = state
        .services
        .workspaces
        .detail(credentials(&headers), space)
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/rename",
    operation_id = "renameWorkspace",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
    ),
    request_body = RenameRequest,
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn rename(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<RenameRequest>,
) -> AppResult<Json<MessageResponse>> {
    let result = state
        .services
        .workspaces
        .rename(credentials(&headers), space, req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/teams",
    operation_id = "createTeam",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
    ),
    request_body = TeamRequest,
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
pub async fn team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<TeamRequest>,
) -> AppResult<Json<MessageResponse>> {
    let result = state
        .services
        .workspaces
        .team(credentials(&headers), space, req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/teams/{team_id}/rename",
    operation_id = "renameTeam",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
        ("team_id" = String, Path),
    ),
    request_body = RenameRequest,
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn rename_team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((space, team)): Path<(String, String)>,
    ApiJson(req): ApiJson<RenameRequest>,
) -> AppResult<Json<MessageResponse>> {
    let result = state
        .services
        .workspaces
        .rename_team(credentials(&headers), space, team, req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/members",
    operation_id = "changeMembership",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
    ),
    request_body = MembershipChange,
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
pub async fn change_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<MembershipChange>,
) -> AppResult<Json<MessageResponse>> {
    let result = state
        .services
        .workspaces
        .change_member(credentials(&headers), space, req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/invitations",
    operation_id = "createInvitation",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
    ),
    request_body = InviteRequest,
    responses(
        (status = 200, body = InviteLink),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<InviteRequest>,
) -> AppResult<Json<InviteLink>> {
    let result = state
        .services
        .workspaces
        .invite(credentials(&headers), space, req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/invitations",
    operation_id = "listInvitations",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
    ),
    responses(
        (status = 200, body = Vec<Invitation>),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn invitations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
) -> AppResult<Json<Vec<Invitation>>> {
    let result = state
        .services
        .workspaces
        .invitations(credentials(&headers), space)
        .await?;
    Ok(Json(result.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/invitations/revoke",
    operation_id = "revokeInvitation",
    tag = "Workspaces",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
    ),
    request_body = RevokeInvitation,
    responses(
        (status = 200, body = MessageResponse),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn revoke_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<RevokeInvitation>,
) -> AppResult<Json<MessageResponse>> {
    let result = state
        .services
        .workspaces
        .revoke_invitation(credentials(&headers), space, req.into())
        .await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/invitations/preview",
    operation_id = "previewInvitation",
    tag = "Accounts",
    request_body = InviteToken,
    responses(
        (status = 200, body = InvitePreview),
        (status = 400, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn preview(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InviteToken>,
) -> AppResult<Json<InvitePreview>> {
    let result = state.services.workspaces.preview(req.into()).await?;
    Ok(Json(result.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/invitations/accept",
    operation_id = "acceptInvitation",
    tag = "Accounts",
    request_body = AcceptInvite,
    responses(
        (status = 200, body = Workspace),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 409, body = crate::http::ApiError),
        (status = 429, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn accept(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<AcceptInvite>,
) -> AppResult<Response> {
    let result = state
        .services
        .workspaces
        .accept(credentials(&headers), req.into())
        .await?;
    Ok(accounts::session_response(&state, result, Workspace::from))
}

#[cfg(test)]
mod tests;
