//! Workspace-scoped issue operations. The workspace share lock coordinates with
//! CP02 removal/disablement; issue row locks serialize edits independently.
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
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, ToSchema, sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum Priority {
    #[default]
    None,
    Urgent,
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct Issue {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub team_id: Uuid,
    pub number: i64,
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub status_id: Uuid,
    pub priority: Priority,
    pub assignee_membership_id: Option<Uuid>,
    pub creator_membership_id: Uuid,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub archived_by_membership_id: Option<Uuid>,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateIssue {
    pub team_id: Uuid,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub status_id: Option<Uuid>,
    #[serde(default)]
    pub priority: Priority,
    pub assignee_membership_id: Option<Uuid>,
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
    #[schema(nullable = false)]
    pub status_id: Option<Uuid>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub priority: Option<Priority>,
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type=Option<Uuid>)]
    pub assignee_membership_id: Option<Option<Uuid>>,
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
    pub team_id: Option<Uuid>,
}
const SELECT: &str = "SELECT i.*,t.key || '-' || i.number::text AS identifier FROM convex.issues i JOIN convex.teams t ON t.id=i.team_id";
fn id(value: &str) -> AppResult<Uuid> {
    value.parse().map_err(|_| AppError::not_found())
}
fn title(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 240 {
        return Err(AppError::validation(
            "Enter an issue title of 1–240 characters.",
        ));
    }
    Ok(value.into())
}
fn description(value: &str) -> AppResult<()> {
    if value.chars().count() > 50000 {
        return Err(AppError::validation(
            "Descriptions can contain at most 50,000 characters.",
        ));
    }
    Ok(())
}
async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    space: Uuid,
    headers: &HeaderMap,
) -> AppResult<Uuid> {
    sqlx::query("SELECT id FROM convex.workspaces WHERE id=$1 FOR SHARE")
        .bind(space)
        .execute(&mut **tx)
        .await?;
    // Avoid CP02's account/session write locks: its admin operations acquire
    // account locks before workspace locks. The shared workspace lock protects
    // membership and account disablement until this operation commits.
    let hash = security::digest(&security::session_token(headers)?);
    let user: Uuid=sqlx::query_scalar("SELECT u.id FROM convex.users u JOIN convex.sessions s ON s.user_id=u.id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>now() AND u.disabled_at IS NULL").bind(hash).fetch_optional(&mut **tx).await?.ok_or_else(AppError::unauthorized)?;
    sqlx::query_scalar("SELECT id FROM convex.workspace_memberships WHERE workspace_id=$1 AND user_id=$2 AND removed_at IS NULL").bind(space).bind(user).fetch_optional(&mut **tx).await?.ok_or_else(AppError::not_found)
}
async fn references(
    tx: &mut Transaction<'_, Postgres>,
    space: Uuid,
    team: Uuid,
    status: Uuid,
    assignee: Option<Uuid>,
) -> AppResult<()> {
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.workflow_statuses WHERE workspace_id=$1 AND team_id=$2 AND id=$3)").bind(space).bind(team).bind(status).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(AppError::validation(
            "Choose a status from this issue's team.",
        ));
    }
    if let Some(member) = assignee {
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.workspace_memberships m JOIN convex.users u ON u.id=m.user_id WHERE m.workspace_id=$1 AND m.id=$2 AND m.removed_at IS NULL AND u.disabled_at IS NULL)").bind(space).bind(member).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(AppError::validation(
                "Choose an active member of this workspace as assignee.",
            ));
        }
    }
    Ok(())
}
async fn load(
    tx: &mut Transaction<'_, Postgres>,
    space: Uuid,
    reference: &str,
    lock: bool,
) -> AppResult<Issue> {
    let query = format!(
        "{SELECT} WHERE i.workspace_id=$1 AND (i.id::text=$2 OR t.key || '-' || i.number::text=$2){}",
        if lock { " FOR UPDATE OF i" } else { "" }
    );
    sqlx::query_as(&query)
        .bind(space)
        .bind(reference)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(AppError::not_found)
}
async fn snapshot(tx: &mut Transaction<'_, Postgres>, issue: &Issue) -> AppResult<Value> {
    let status: String =
        sqlx::query_scalar("SELECT name FROM convex.workflow_statuses WHERE id=$1")
            .bind(issue.status_id)
            .fetch_one(&mut **tx)
            .await?;
    let assignee:Option<String>=sqlx::query_scalar("SELECT u.display_name FROM convex.workspace_memberships m JOIN convex.users u ON u.id=m.user_id WHERE m.id=$1").bind(issue.assignee_membership_id).fetch_optional(&mut **tx).await?;
    Ok(
        json!({"title":issue.title,"description":issue.description,"status":{"id":issue.status_id,"name":status},"priority":issue.priority,"assignee":{"id":issue.assignee_membership_id,"name":assignee},"archived_at":issue.archived_at,"version":issue.version}),
    )
}
async fn activity(
    tx: &mut Transaction<'_, Postgres>,
    issue: &Issue,
    actor: Uuid,
    kind: &str,
    before: Option<Value>,
) -> AppResult<()> {
    let after = snapshot(tx, issue).await?;
    let actor_name:String=sqlx::query_scalar("SELECT u.display_name FROM convex.workspace_memberships m JOIN convex.users u ON u.id=m.user_id WHERE m.id=$1").bind(actor).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO convex.issue_activity(id,workspace_id,issue_id,actor_membership_id,kind,payload) VALUES($1,$2,$3,$4,$5,$6)").bind(Uuid::new_v4()).bind(issue.workspace_id).bind(issue.id).bind(actor).bind(kind).bind(json!({"actor_name":actor_name,"before":before,"after":after})).execute(&mut **tx).await?;
    Ok(())
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/issues",operation_id="createIssue",tag="Issues",security(("cookieSession"=[])),params(("workspace_id"=String,Path)),request_body=CreateIssue,responses((status=201,body=Issue),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<CreateIssue>,
) -> AppResult<Response> {
    let space = id(&space)?;
    let title = title(&req.title)?;
    description(&req.description)?;
    let mut tx = state.pool.begin().await?;
    let actor = authorize(&mut tx, space, &headers).await?;
    let status = if let Some(status) = req.status_id {
        status
    } else {
        sqlx::query_scalar("SELECT id FROM convex.workflow_statuses WHERE workspace_id=$1 AND team_id=$2 AND category='unstarted' ORDER BY position,id LIMIT 1").bind(space).bind(req.team_id).fetch_optional(&mut *tx).await?.ok_or_else(||AppError::validation("Choose a team with an unstarted status."))?
    };
    references(
        &mut tx,
        space,
        req.team_id,
        status,
        req.assignee_membership_id,
    )
    .await?;
    let number:i64=sqlx::query_scalar("UPDATE convex.teams SET next_issue_number=next_issue_number+1 WHERE workspace_id=$1 AND id=$2 RETURNING next_issue_number-1").bind(space).bind(req.team_id).fetch_optional(&mut *tx).await?.ok_or_else(||AppError::validation("Choose a team in this workspace."))?;
    let issue_id = Uuid::new_v4();
    sqlx::query("INSERT INTO convex.issues(id,workspace_id,team_id,number,title,description,status_id,priority,assignee_membership_id,creator_membership_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)").bind(issue_id).bind(space).bind(req.team_id).bind(number).bind(title).bind(req.description).bind(status).bind(req.priority).bind(req.assignee_membership_id).bind(actor).execute(&mut *tx).await?;
    let issue = load(&mut tx, space, &issue_id.to_string(), false).await?;
    activity(&mut tx, &issue, actor, "created", None).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(issue)).into_response())
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/issues",operation_id="listIssues",tag="Issues",security(("cookieSession"=[])),params(("workspace_id"=String,Path),IssueListQuery),responses((status=200,body=Vec<Issue>),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    query: Result<Query<IssueListQuery>, axum::extract::rejection::QueryRejection>,
) -> AppResult<Json<Vec<Issue>>> {
    let Query(query) = query.map_err(|_| {
        AppError::validation("Use a valid team UUID and archived=true or archived=false.")
    })?;
    let space = id(&space)?;
    let mut tx = state.pool.begin().await?;
    authorize(&mut tx, space, &headers).await?;
    if let Some(team) = query.team_id {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM convex.teams WHERE workspace_id=$1 AND id=$2)",
        )
        .bind(space)
        .bind(team)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(AppError::not_found());
        }
    }
    let issues=sqlx::query_as(&format!("{SELECT} WHERE i.workspace_id=$1 AND (i.archived_at IS NOT NULL)=$2 AND ($3::uuid IS NULL OR i.team_id=$3) ORDER BY i.created_at DESC,i.id DESC")).bind(space).bind(query.archived).bind(query.team_id).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(issues))
}
#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/issues/{issue_ref}",operation_id="getIssue",tag="Issues",security(("cookieSession"=[])),params(("workspace_id"=String,Path),("issue_ref"=String,Path,description="Issue UUID or exact human identifier, including archived issues")),responses((status=200,body=Issue),(status=401,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((space, reference)): Path<(String, String)>,
) -> AppResult<Json<Issue>> {
    let space = id(&space)?;
    let mut tx = state.pool.begin().await?;
    authorize(&mut tx, space, &headers).await?;
    let issue = load(&mut tx, space, &reference, false).await?;
    tx.commit().await?;
    Ok(Json(issue))
}
#[utoipa::path(patch,path="/api/v1/workspaces/{workspace_id}/issues/{issue_ref}",operation_id="updateIssue",tag="Issues",security(("cookieSession"=[])),params(("workspace_id"=String,Path),("issue_ref"=String,Path)),request_body=IssuePatch,responses((status=200,body=Issue),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=409,body=IssueConflict),(status=503,body=crate::http::ApiError)))]
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((space, reference)): Path<(String, String)>,
    ApiJson(req): ApiJson<IssuePatch>,
) -> AppResult<Response> {
    let space = id(&space)?;
    if req.expected_version < 1 {
        return Err(AppError::validation("Supply a positive expected_version."));
    }
    let mut tx = state.pool.begin().await?;
    let actor = authorize(&mut tx, space, &headers).await?;
    let old = load(&mut tx, space, &reference, true).await?;
    if old.version != req.expected_version {
        tx.commit().await?;
        return Ok((
            StatusCode::CONFLICT,
            Json(IssueConflict {
                code: crate::http::ErrorCode::Conflict,
                message: "This issue changed. Review the current issue before saving again.".into(),
                current_issue: old,
            }),
        )
            .into_response());
    }
    let properties = req.title.is_some()
        || req.description.is_some()
        || req.status_id.is_some()
        || req.priority.is_some()
        || req.assignee_membership_id.is_some();
    if req.archived.is_some() && properties {
        return Err(AppError::validation(
            "Archive or restore separately from property edits.",
        ));
    }
    if old.archived_at.is_some() && properties {
        return Err(AppError::validation(
            "Restore this issue before editing it.",
        ));
    }
    let mut next = old.clone();
    if let Some(value) = req.title {
        next.title = title(&value)?;
    }
    if let Some(value) = req.description {
        description(&value)?;
        next.description = value;
    }
    if let Some(value) = req.status_id {
        next.status_id = value;
    }
    if let Some(value) = req.priority {
        next.priority = value;
    }
    if let Some(value) = req.assignee_membership_id {
        next.assignee_membership_id = value;
    }
    // Only validate changed references: historical assignees remain resolvable.
    if req.status_id.is_some() || req.assignee_membership_id.is_some() {
        references(
            &mut tx,
            space,
            next.team_id,
            next.status_id,
            req.assignee_membership_id.flatten(),
        )
        .await?;
    }
    let archived = req.archived.unwrap_or(old.archived_at.is_some());
    let changed = next.title != old.title
        || next.description != old.description
        || next.status_id != old.status_id
        || next.priority != old.priority
        || next.assignee_membership_id != old.assignee_membership_id
        || archived != old.archived_at.is_some();
    if !changed {
        tx.commit().await?;
        return Ok(Json(old).into_response());
    }
    let before = snapshot(&mut tx, &old).await?;
    sqlx::query("UPDATE convex.issues SET title=$3,description=$4,status_id=$5,priority=$6,assignee_membership_id=$7,version=version+1,updated_at=now(),archived_at=CASE WHEN $8 THEN coalesce(archived_at,now()) ELSE NULL END,archived_by_membership_id=CASE WHEN $8 THEN coalesce(archived_by_membership_id,$9) ELSE NULL END WHERE workspace_id=$1 AND id=$2").bind(space).bind(old.id).bind(next.title).bind(next.description).bind(next.status_id).bind(next.priority).bind(next.assignee_membership_id).bind(archived).bind(actor).execute(&mut *tx).await?;
    let issue = load(&mut tx, space, &old.id.to_string(), false).await?;
    let kind = if archived != old.archived_at.is_some() {
        if archived { "archived" } else { "restored" }
    } else {
        "updated"
    };
    activity(&mut tx, &issue, actor, kind, Some(before)).await?;
    tx.commit().await?;
    Ok(Json(issue).into_response())
}

/// Caller holds the workspace exclusively (CP02 membership administration).
/// Includes archived issues so departed members retain no active assignments.
pub async fn clear_assignments(
    tx: &mut Transaction<'_, Postgres>,
    space: Uuid,
    member: Uuid,
    actor: Uuid,
) -> AppResult<()> {
    let rows:Vec<Issue>=sqlx::query_as(&format!("{SELECT} WHERE i.workspace_id=$1 AND i.assignee_membership_id=$2 ORDER BY i.id FOR UPDATE OF i")).bind(space).bind(member).fetch_all(&mut **tx).await?;
    for old in rows {
        let before = snapshot(tx, &old).await?;
        sqlx::query("UPDATE convex.issues SET assignee_membership_id=NULL,version=version+1,updated_at=now() WHERE id=$1").bind(old.id).execute(&mut **tx).await?;
        let issue = load(tx, space, &old.id.to_string(), false).await?;
        activity(tx, &issue, actor, "updated", Some(before)).await?;
    }
    Ok(())
}
