//! Issue HTTP contract and request translation. Domain rules and transactions
//! live in convex-issues and convex-app respectively.
use crate::{
    http::{ApiJson, AppError, AppResult, AppState},
    security,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use convex_app::UpdateIssueResult;
use convex_ids::{IssueId, MembershipId, TeamId, WorkflowStatusId, WorkspaceId};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    #[default]
    None,
    Urgent,
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct Issue {
    #[schema(inline)]
    pub id: IssueId,
    #[schema(inline)]
    pub workspace_id: WorkspaceId,
    #[schema(inline)]
    pub team_id: TeamId,
    pub number: i64,
    pub identifier: String,
    pub title: String,
    pub description: String,
    #[schema(inline)]
    pub status_id: WorkflowStatusId,
    pub priority: Priority,
    #[schema(value_type = Option<String>, format = Uuid)]
    pub assignee_membership_id: Option<MembershipId>,
    #[schema(inline)]
    pub creator_membership_id: MembershipId,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    #[schema(value_type = Option<String>, format = Uuid)]
    pub archived_by_membership_id: Option<MembershipId>,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateIssue {
    #[schema(inline)]
    pub team_id: TeamId,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[schema(value_type = Option<String>, format = Uuid)]
    pub status_id: Option<WorkflowStatusId>,
    #[serde(default)]
    pub priority: Priority,
    #[schema(value_type = Option<String>, format = Uuid)]
    pub assignee_membership_id: Option<MembershipId>,
}
// serde's default Option collapses explicit null and omission. These helpers
// preserve that distinction and reject null for non-nullable patch properties.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IssuePatch {
    pub expected_version: i64,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false, inline)]
    pub status_id: Option<WorkflowStatusId>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub priority: Option<Priority>,
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type = Option<String>, format = Uuid)]
    pub assignee_membership_id: Option<Option<MembershipId>>,
    /// Archive/restore is exclusive of property edits.
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub archived: Option<bool>,
}
#[derive(Serialize, ToSchema)]
pub struct IssueConflict {
    pub code: crate::http::ErrorCode,
    pub message: String,
    pub current_issue: Issue,
}
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct IssueListQuery {
    /// false lists active issues; true lists archived issues only.
    #[serde(default)]
    pub archived: bool,
    /// Restrict the collection to a team in this workspace.
    #[param(inline)]
    pub team_id: Option<TeamId>,
}
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/issues",
    operation_id = "createIssue",
    tag = "Issues",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
    ),
    request_body = CreateIssue,
    responses(
        (status = 201, body = Issue),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(request): ApiJson<CreateIssue>,
) -> AppResult<Response> {
    let workspace = workspace_id(&space)?;
    let token_digest = session_digest(&headers);
    let result = state
        .services
        .issues
        .create(workspace, token_digest.as_deref(), request.into())
        .await?;
    Ok((StatusCode::CREATED, Json(Issue::from(result.value))).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/issues",
    operation_id = "listIssues",
    tag = "Issues",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
        IssueListQuery,
    ),
    responses(
        (status = 200, body = Vec<Issue>),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    query: Result<Query<IssueListQuery>, axum::extract::rejection::QueryRejection>,
) -> AppResult<Json<Vec<Issue>>> {
    let Query(query) = query.map_err(|_| {
        AppError::validation("Use a valid team UUID and archived=true or archived=false.")
    })?;
    let workspace = workspace_id(&space)?;
    let token_digest = session_digest(&headers);
    let filter = convex_issues::repository::IssueFilter {
        archived: query.archived,
        team_id: query.team_id,
    };
    let result = state
        .services
        .issues
        .list(workspace, token_digest.as_deref(), filter)
        .await?;
    Ok(Json(result.value.into_iter().map(Issue::from).collect()))
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/issues/{issue_ref}",
    operation_id = "getIssue",
    tag = "Issues",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
        (
            "issue_ref" = String,
            Path,
            description = "Issue UUID or exact human identifier, including archived issues",
        ),
    ),
    responses(
        (status = 200, body = Issue),
        (status = 401, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((space, reference)): Path<(String, String)>,
) -> AppResult<Json<Issue>> {
    let workspace = workspace_id(&space)?;
    let token_digest = session_digest(&headers);
    let result = state
        .services
        .issues
        .detail(workspace, token_digest.as_deref(), &reference)
        .await?;
    Ok(Json(result.value.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/workspaces/{workspace_id}/issues/{issue_ref}",
    operation_id = "updateIssue",
    tag = "Issues",
    security(("cookieSession" = [])),
    params(
        ("workspace_id" = String, Path),
        ("issue_ref" = String, Path),
    ),
    request_body = IssuePatch,
    responses(
        (status = 200, body = Issue),
        (status = 400, body = crate::http::ApiError),
        (status = 401, body = crate::http::ApiError),
        (status = 403, body = crate::http::ApiError),
        (status = 404, body = crate::http::ApiError),
        (status = 409, body = IssueConflict),
        (status = 503, body = crate::http::ApiError),
    ),
)]
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((space, reference)): Path<(String, String)>,
    ApiJson(request): ApiJson<IssuePatch>,
) -> AppResult<Response> {
    let workspace = workspace_id(&space)?;
    let token_digest = session_digest(&headers);
    let result = state
        .services
        .issues
        .update(
            workspace,
            token_digest.as_deref(),
            &reference,
            request.into(),
        )
        .await?;
    match result.value {
        UpdateIssueResult::Updated(issue) => Ok(Json(Issue::from(issue)).into_response()),
        UpdateIssueResult::Conflict(issue) => Ok((
            StatusCode::CONFLICT,
            Json(IssueConflict {
                code: crate::http::ErrorCode::Conflict,
                message: "This issue changed. Review the current issue before saving again.".into(),
                current_issue: issue.into(),
            }),
        )
            .into_response()),
    }
}

fn session_digest(headers: &HeaderMap) -> Option<String> {
    security::session_token(headers)
        .ok()
        .map(|token| security::digest(&token))
}

fn workspace_id(value: &str) -> AppResult<WorkspaceId> {
    value.parse().map_err(|_| AppError::not_found())
}

impl From<CreateIssue> for convex_issues::NewIssue {
    fn from(request: CreateIssue) -> Self {
        Self {
            team_id: request.team_id,
            title: request.title,
            description: request.description,
            status_id: request.status_id,
            priority: request.priority.into(),
            assignee_membership_id: request.assignee_membership_id,
        }
    }
}

impl From<IssuePatch> for convex_issues::IssuePatch {
    fn from(request: IssuePatch) -> Self {
        Self {
            expected_version: request.expected_version,
            title: request.title,
            description: request.description,
            status_id: request.status_id,
            priority: request.priority.map(Into::into),
            assignee_membership_id: request.assignee_membership_id,
            archived: request.archived,
        }
    }
}

impl From<convex_issues::Issue> for Issue {
    fn from(issue: convex_issues::Issue) -> Self {
        Self {
            id: issue.id,
            workspace_id: issue.workspace_id,
            team_id: issue.team_id,
            number: issue.number,
            identifier: issue.identifier,
            title: issue.title,
            description: issue.description,
            status_id: issue.status_id,
            priority: issue.priority.into(),
            assignee_membership_id: issue.assignee_membership_id,
            creator_membership_id: issue.creator_membership_id,
            version: issue.version,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            archived_at: issue.archived_at,
            archived_by_membership_id: issue.archived_by_membership_id,
        }
    }
}

impl From<Priority> for convex_issues::Priority {
    fn from(priority: Priority) -> Self {
        match priority {
            Priority::None => Self::None,
            Priority::Urgent => Self::Urgent,
            Priority::High => Self::High,
            Priority::Medium => Self::Medium,
            Priority::Low => Self::Low,
        }
    }
}

impl From<convex_issues::Priority> for Priority {
    fn from(priority: convex_issues::Priority) -> Self {
        match priority {
            convex_issues::Priority::None => Self::None,
            convex_issues::Priority::Urgent => Self::Urgent,
            convex_issues::Priority::High => Self::High,
            convex_issues::Priority::Medium => Self::Medium,
            convex_issues::Priority::Low => Self::Low,
        }
    }
}

#[cfg(test)]
mod tests;
