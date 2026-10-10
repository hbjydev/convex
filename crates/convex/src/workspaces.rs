use crate::{
    accounts::{self, MessageResponse},
    http::{ApiJson, AppError, AppResult, AppState},
    security as sec,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: String,
}
#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct Member {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub role: String,
    pub disabled: bool,
}
#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct WorkflowStatus {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub position: i32,
}
#[derive(Serialize, ToSchema)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub key: String,
    pub statuses: Vec<WorkflowStatus>,
}
#[derive(Serialize, ToSchema)]
pub struct WorkspaceDetail {
    pub workspace: Workspace,
    pub members: Vec<Member>,
    pub teams: Vec<Team>,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateWorkspace {
    pub name: String,
    pub slug: String,
    pub team_name: String,
    pub team_key: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TeamRequest {
    pub name: String,
    pub key: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameRequest {
    pub name: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MembershipChange {
    pub membership_id: Uuid,
    pub role: Option<String>,
    pub remove: bool,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InviteRequest {
    pub role: String,
    pub deliver_to: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct InviteLink {
    pub id: Uuid,
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub email_delivered: bool,
}
#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct Invitation {
    pub id: Uuid,
    pub role: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RevokeInvitation {
    pub invitation_id: Uuid,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InviteToken {
    pub token: String,
}
#[derive(Serialize, ToSchema)]
pub struct InvitePreview {
    pub workspace_name: String,
    pub role: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptInvite {
    pub token: String,
    pub email: Option<String>,
    pub password: Option<String>,
    pub display_name: Option<String>,
}
#[derive(sqlx::FromRow)]
struct ValidInvite {
    id: Uuid,
    workspace_id: Uuid,
    role: String,
    workspace_name: String,
    workspace_slug: String,
}

pub async fn list_for_user(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
) -> AppResult<Vec<Workspace>> {
    Ok(sqlx::query_as("SELECT w.id,w.name,w.slug,m.role FROM convex.workspaces w JOIN convex.workspace_memberships m ON m.workspace_id=w.id WHERE m.user_id=$1 AND m.removed_at IS NULL ORDER BY w.created_at,w.id").bind(user).fetch_all(&mut **tx).await?)
}

pub async fn create_workspace(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    name: String,
    slug: String,
    team_name: String,
    team_key: String,
) -> AppResult<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO convex.workspaces (id,name,slug) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(name)
        .bind(slug)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO convex.workspace_memberships (id,workspace_id,user_id,role) VALUES ($1,$2,$3,'owner')").bind(Uuid::new_v4()).bind(id).bind(user).execute(&mut **tx).await?;
    create_team(tx, id, team_name, team_key).await?;
    Ok(id)
}
async fn create_team(
    tx: &mut Transaction<'_, Postgres>,
    workspace: Uuid,
    name: String,
    key: String,
) -> AppResult<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO convex.teams (id,workspace_id,name,key) VALUES ($1,$2,$3,$4)")
        .bind(id)
        .bind(workspace)
        .bind(name)
        .bind(key)
        .execute(&mut **tx)
        .await?;
    for (position, (name, category)) in [
        ("Backlog", "backlog"),
        ("Todo", "unstarted"),
        ("In Progress", "started"),
        ("Done", "completed"),
        ("Canceled", "canceled"),
    ]
    .into_iter()
    .enumerate()
    {
        sqlx::query("INSERT INTO convex.workflow_statuses (id,workspace_id,team_id,name,category,position) VALUES ($1,$2,$3,$4,$5,$6)").bind(Uuid::new_v4()).bind(workspace).bind(id).bind(name).bind(category).bind(position as i32).execute(&mut **tx).await?;
    }
    Ok(id)
}
async fn membership(
    tx: &mut Transaction<'_, Postgres>,
    workspace: Uuid,
    user: Uuid,
) -> AppResult<(Uuid, String)> {
    sqlx::query("SELECT id FROM convex.workspaces WHERE id=$1 FOR UPDATE")
        .bind(workspace)
        .execute(&mut **tx)
        .await?;
    sqlx::query_as("SELECT id,role FROM convex.workspace_memberships WHERE workspace_id=$1 AND user_id=$2 AND removed_at IS NULL").bind(workspace).bind(user).fetch_optional(&mut **tx).await?.ok_or_else(AppError::not_found)
}
fn manage(role: &str) -> AppResult<()> {
    if role == "owner" || role == "admin" {
        Ok(())
    } else {
        Err(AppError::forbidden())
    }
}
fn role(value: &str, owners: bool) -> AppResult<()> {
    if matches!(value, "admin" | "member") || (owners && value == "owner") {
        Ok(())
    } else {
        Err(AppError::validation("Choose an allowed workspace role."))
    }
}
pub async fn protect_last_owner(
    tx: &mut Transaction<'_, Postgres>,
    workspace: Uuid,
    user: Uuid,
) -> AppResult<()> {
    let owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.workspace_memberships WHERE workspace_id=$1 AND user_id=$2 AND role='owner' AND removed_at IS NULL)").bind(workspace).bind(user).fetch_one(&mut **tx).await?;
    if owner {
        let other: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.workspace_memberships m JOIN convex.users u ON m.user_id=u.id WHERE m.workspace_id=$1 AND m.user_id<>$2 AND m.role='owner' AND m.removed_at IS NULL AND u.disabled_at IS NULL)").bind(workspace).bind(user).fetch_one(&mut **tx).await?;
        if !other {
            return Err(AppError::conflict(
                "Keep at least one active workspace owner.",
            ));
        }
    }
    Ok(())
}
fn workspace_id(value: &str) -> AppResult<Uuid> {
    value.parse().map_err(|_| AppError::not_found())
}

#[utoipa::path(post,path="/api/v1/workspaces",operation_id="createWorkspace",tag="Workspaces",security(("cookieSession"=[])),request_body=CreateWorkspace,responses((status=200,body=Workspace),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<CreateWorkspace>,
) -> AppResult<Json<Workspace>> {
    let name = sec::name(&req.name)?;
    let slug = sec::workspace_slug(&req.slug)?;
    let team_name = sec::name(&req.team_name)?;
    let team_key = sec::team_key(&req.team_key)?;
    let mut tx = accounts::transaction(&state.pool).await?;
    let actor = accounts::authenticate(&mut tx, &headers).await?;
    if !actor.is_instance_admin {
        return Err(AppError::forbidden());
    }
    let id = create_workspace(
        &mut tx,
        actor.id,
        name.clone(),
        slug.clone(),
        team_name,
        team_key,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(Workspace {
        id,
        name,
        slug,
        role: "owner".into(),
    }))
}

#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}",operation_id="getWorkspace",tag="Workspaces",security(("cookieSession"=[])),params(("workspace_id"=String,Path)),responses((status=200,body=WorkspaceDetail),(status=401,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
) -> AppResult<Json<WorkspaceDetail>> {
    let space = workspace_id(&space)?;
    let mut tx = accounts::transaction(&state.pool).await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    let (_, role) = membership(&mut tx, space, user.id).await?;
    let (name, slug): (String, String) =
        sqlx::query_as("SELECT name,slug FROM convex.workspaces WHERE id=$1")
            .bind(space)
            .fetch_one(&mut *tx)
            .await?;
    let members=sqlx::query_as("SELECT m.id,m.user_id,u.display_name,m.role,u.disabled_at IS NOT NULL AS disabled FROM convex.workspace_memberships m JOIN convex.users u ON m.user_id=u.id WHERE m.workspace_id=$1 AND m.removed_at IS NULL ORDER BY m.joined_at,m.id").bind(space).fetch_all(&mut *tx).await?;
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id,name,key FROM convex.teams WHERE workspace_id=$1 ORDER BY created_at,id",
    )
    .bind(space)
    .fetch_all(&mut *tx)
    .await?;
    let mut teams = Vec::new();
    for (id, name, key) in rows {
        let statuses=sqlx::query_as("SELECT id,name,category,position FROM convex.workflow_statuses WHERE team_id=$1 ORDER BY position,id").bind(id).fetch_all(&mut *tx).await?;
        teams.push(Team {
            id,
            name,
            key,
            statuses,
        });
    }
    tx.commit().await?;
    Ok(Json(WorkspaceDetail {
        workspace: Workspace {
            id: space,
            name,
            slug,
            role,
        },
        members,
        teams,
    }))
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/rename",operation_id="renameWorkspace",tag="Workspaces",security(("cookieSession"=[])),params(("workspace_id"=String,Path)),request_body=RenameRequest,responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn rename(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<RenameRequest>,
) -> AppResult<Json<MessageResponse>> {
    let space = workspace_id(&space)?;
    let name = sec::name(&req.name)?;
    let mut tx = accounts::transaction(&state.pool).await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    manage(&membership(&mut tx, space, user.id).await?.1)?;
    sqlx::query("UPDATE convex.workspaces SET name=$2,updated_at=now() WHERE id=$1")
        .bind(space)
        .bind(name)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(MessageResponse {
        message: "Workspace renamed.".into(),
    }))
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/teams",operation_id="createTeam",tag="Workspaces",security(("cookieSession"=[])),params(("workspace_id"=String,Path)),request_body=TeamRequest,responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<TeamRequest>,
) -> AppResult<Json<MessageResponse>> {
    let space = workspace_id(&space)?;
    let name = sec::name(&req.name)?;
    let key = sec::team_key(&req.key)?;
    let mut tx = accounts::transaction(&state.pool).await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    manage(&membership(&mut tx, space, user.id).await?.1)?;
    create_team(&mut tx, space, name, key).await?;
    tx.commit().await?;
    Ok(Json(MessageResponse {
        message: "Team created with its initial workflow.".into(),
    }))
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/teams/{team_id}/rename",operation_id="renameTeam",tag="Workspaces",security(("cookieSession"=[])),params(("workspace_id"=String,Path),("team_id"=String,Path)),request_body=RenameRequest,responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn rename_team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((space, team)): Path<(String, String)>,
    ApiJson(req): ApiJson<RenameRequest>,
) -> AppResult<Json<MessageResponse>> {
    let space = workspace_id(&space)?;
    let team = workspace_id(&team)?;
    let name = sec::name(&req.name)?;
    let mut tx = accounts::transaction(&state.pool).await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    manage(&membership(&mut tx, space, user.id).await?.1)?;
    if sqlx::query(
        "UPDATE convex.teams SET name=$3,updated_at=now() WHERE id=$2 AND workspace_id=$1",
    )
    .bind(space)
    .bind(team)
    .bind(name)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 0
    {
        return Err(AppError::not_found());
    }
    tx.commit().await?;
    Ok(Json(MessageResponse {
        message: "Team renamed. Its key stays the same.".into(),
    }))
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/members",operation_id="changeMembership",tag="Workspaces",security(("cookieSession"=[])),params(("workspace_id"=String,Path)),request_body=MembershipChange,responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn change_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<MembershipChange>,
) -> AppResult<Json<MessageResponse>> {
    let space = workspace_id(&space)?;
    if req.remove == req.role.is_some() {
        return Err(AppError::validation(
            "Choose either a role change or membership removal.",
        ));
    }
    if let Some(value) = &req.role {
        role(value, true)?;
    }
    let mut tx = accounts::transaction(&state.pool).await?;
    let actor = accounts::authenticate(&mut tx, &headers).await?;
    let (_, actor_role) = membership(&mut tx, space, actor.id).await?;
    manage(&actor_role)?;
    let (user,target_role):(Uuid,String)=sqlx::query_as("SELECT user_id,role FROM convex.workspace_memberships WHERE id=$1 AND workspace_id=$2 AND removed_at IS NULL").bind(req.membership_id).bind(space).fetch_optional(&mut *tx).await?.ok_or_else(AppError::not_found)?;
    // Admins manage ordinary members; only owners grant or change administration.
    if actor_role != "owner"
        && (target_role != "member" || req.role.as_deref().is_some_and(|r| r != "member"))
    {
        return Err(AppError::forbidden());
    }
    if req.remove || req.role.as_deref() != Some("owner") {
        protect_last_owner(&mut tx, space, user).await?;
    }
    if req.remove {
        sqlx::query("UPDATE convex.workspace_memberships SET removed_at=now() WHERE id=$1")
            .bind(req.membership_id)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("UPDATE convex.workspace_memberships SET role=$2 WHERE id=$1")
            .bind(req.membership_id)
            .bind(req.role)
            .execute(&mut *tx)
            .await?;
    }
    accounts::revoke_all(&mut tx, user).await?;
    tx.commit().await?;
    Ok(Json(MessageResponse {
        message: "Membership updated. This person's sessions were revoked.".into(),
    }))
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/invitations",operation_id="createInvitation",tag="Workspaces",security(("cookieSession"=[])),params(("workspace_id"=String,Path)),request_body=InviteRequest,responses((status=200,body=InviteLink),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<InviteRequest>,
) -> AppResult<Json<InviteLink>> {
    let space = workspace_id(&space)?;
    role(&req.role, false)?;
    let to = req.deliver_to.as_deref().map(sec::email).transpose()?;
    if to.is_some() && state.security.mail.is_none() {
        return Err(AppError::validation(
            "Email delivery is unavailable. Create a link to share directly.",
        ));
    }
    let mut tx = accounts::transaction(&state.pool).await?;
    let actor = accounts::authenticate(&mut tx, &headers).await?;
    let (membership, actor_role) = membership(&mut tx, space, actor.id).await?;
    manage(&actor_role)?;
    if req.role == "admin" && actor_role != "owner" {
        return Err(AppError::forbidden());
    }
    let token = sec::token();
    let id = Uuid::new_v4();
    let expires_at=sqlx::query_scalar("INSERT INTO convex.invitations (id,workspace_id,inviter_membership_id,role,token_hash) VALUES ($1,$2,$3,$4,$5) RETURNING expires_at").bind(id).bind(space).bind(membership).bind(req.role).bind(sec::digest(&token)).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    let url = format!("{}/invite#{}", state.security.origin, token);
    let delivered = if let Some(to) = to {
        match state.security.send(&to,"Join your workspace on Convex",format!("Join using this single-use invitation within 7 days:\n{url}\nOnly share this link with the intended teammate.")).await { Ok(sent)=>sent,Err(error)=>{tracing::error!(%error,"Invitation delivery failed; administrator can share the returned link");false} }
    } else {
        false
    };
    Ok(Json(InviteLink {
        id,
        url,
        expires_at,
        email_delivered: delivered,
    }))
}

#[utoipa::path(get,path="/api/v1/workspaces/{workspace_id}/invitations",operation_id="listInvitations",tag="Workspaces",security(("cookieSession"=[])),params(("workspace_id"=String,Path)),responses((status=200,body=Vec<Invitation>),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn invitations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
) -> AppResult<Json<Vec<Invitation>>> {
    let space = workspace_id(&space)?;
    let mut tx = accounts::transaction(&state.pool).await?;
    let actor = accounts::authenticate(&mut tx, &headers).await?;
    manage(&membership(&mut tx, space, actor.id).await?.1)?;
    let invitations=sqlx::query_as("SELECT id,role,expires_at,accepted_at,revoked_at FROM convex.invitations WHERE workspace_id=$1 ORDER BY created_at DESC,id").bind(space).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(invitations))
}

#[utoipa::path(post,path="/api/v1/workspaces/{workspace_id}/invitations/revoke",operation_id="revokeInvitation",tag="Workspaces",security(("cookieSession"=[])),params(("workspace_id"=String,Path)),request_body=RevokeInvitation,responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn revoke_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(space): Path<String>,
    ApiJson(req): ApiJson<RevokeInvitation>,
) -> AppResult<Json<MessageResponse>> {
    let space = workspace_id(&space)?;
    let mut tx = accounts::transaction(&state.pool).await?;
    let actor = accounts::authenticate(&mut tx, &headers).await?;
    manage(&membership(&mut tx, space, actor.id).await?.1)?;
    if sqlx::query("UPDATE convex.invitations SET revoked_at=now() WHERE id=$1 AND workspace_id=$2")
        .bind(req.invitation_id)
        .bind(space)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        == 0
    {
        return Err(AppError::not_found());
    }
    tx.commit().await?;
    Ok(Json(MessageResponse {
        message: "Invitation revoked.".into(),
    }))
}

async fn valid_invite(tx: &mut Transaction<'_, Postgres>, token: &str) -> AppResult<ValidInvite> {
    // Recheck inviter authority at use time, including account disablement.
    sqlx::query_as("SELECT i.id,i.workspace_id,i.role,w.name AS workspace_name,w.slug AS workspace_slug FROM convex.invitations i JOIN convex.workspaces w ON w.id=i.workspace_id JOIN convex.workspace_memberships m ON m.id=i.inviter_membership_id JOIN convex.users u ON u.id=m.user_id WHERE i.token_hash=$1 AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>now() AND m.removed_at IS NULL AND u.disabled_at IS NULL AND (m.role='owner' OR (m.role='admin' AND i.role='member')) FOR UPDATE OF i,w,m,u").bind(sec::digest(token)).fetch_optional(&mut **tx).await?.ok_or_else(||AppError::validation("This invitation is invalid, expired, or already used. Ask a workspace admin for a new link."))
}

#[utoipa::path(post,path="/api/v1/invitations/preview",operation_id="previewInvitation",tag="Accounts",request_body=InviteToken,responses((status=200,body=InvitePreview),(status=400,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn preview(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InviteToken>,
) -> AppResult<Json<InvitePreview>> {
    sec::rate_limit(
        &state.pool,
        &format!("invitation-preview:{}", sec::digest(&req.token)),
    )
    .await?;
    let mut tx = accounts::transaction(&state.pool).await?;
    let invitation = valid_invite(&mut tx, &req.token).await?;
    tx.commit().await?;
    Ok(Json(InvitePreview {
        workspace_name: invitation.workspace_name,
        role: invitation.role,
    }))
}

#[utoipa::path(post,path="/api/v1/invitations/accept",operation_id="acceptInvitation",tag="Accounts",request_body=AcceptInvite,responses((status=200,body=Workspace),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn accept(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<AcceptInvite>,
) -> AppResult<Response> {
    sec::rate_limit(
        &state.pool,
        &format!("invitation-accept:{}", sec::digest(&req.token)),
    )
    .await?;
    let credentials = if req.email.is_none() && req.password.is_none() && req.display_name.is_none()
    {
        None
    } else {
        Some((
            sec::email(
                req.email
                    .as_deref()
                    .ok_or_else(|| AppError::validation("Enter your email."))?,
            )?,
            sec::hash_password(
                req.password
                    .ok_or_else(|| AppError::validation("Enter your password."))?,
            )
            .await?,
            sec::name(
                req.display_name
                    .as_deref()
                    .ok_or_else(|| AppError::validation("Enter your name."))?,
            )?,
        ))
    };
    let mut tx = accounts::transaction(&state.pool).await?;
    let invitation = valid_invite(&mut tx, &req.token).await?;
    let user = if let Some((email, hash, name)) = credentials {
        accounts::create_local(&mut tx, email, hash, name, false).await?
    } else {
        accounts::authenticate(&mut tx, &headers).await?
    };
    // Existing active membership is not demoted/promoted by accepting another invitation.
    let current:Option<bool>=sqlx::query_scalar("SELECT removed_at IS NULL FROM convex.workspace_memberships WHERE workspace_id=$1 AND user_id=$2").bind(invitation.workspace_id).bind(user.id).fetch_optional(&mut *tx).await?;
    if current == Some(true) {
        return Err(AppError::conflict("You already belong to this workspace."));
    }
    sqlx::query("INSERT INTO convex.workspace_memberships (id,workspace_id,user_id,role) VALUES ($1,$2,$3,$4) ON CONFLICT (workspace_id,user_id) DO UPDATE SET removed_at=NULL,role=EXCLUDED.role,joined_at=now()").bind(Uuid::new_v4()).bind(invitation.workspace_id).bind(user.id).bind(&invitation.role).execute(&mut *tx).await?;
    sqlx::query("UPDATE convex.invitations SET accepted_at=now(),accepted_by=$2 WHERE id=$1")
        .bind(invitation.id)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    accounts::revoke_all(&mut tx, user.id).await?;
    let token = sec::new_session(&mut tx, user.id).await?;
    tx.commit().await?;
    Ok(accounts::signed_in(
        &state,
        &token,
        Workspace {
            id: invitation.workspace_id,
            name: invitation.workspace_name,
            slug: invitation.workspace_slug,
            role: invitation.role,
        },
    ))
}
