use crate::workspaces_model::*;
use convex_core::{Error as AppError, Result as AppResult};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub async fn list_for_user(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
) -> AppResult<Vec<Workspace>> {
    Ok(sqlx::query_as(
        "SELECT w.id,w.name,w.slug,m.role
             FROM convex.workspaces w
             JOIN convex.workspace_memberships m ON m.workspace_id=w.id
             WHERE m.user_id=$1
             AND m.removed_at IS NULL
             ORDER BY w.created_at,w.id",
    )
    .bind(user)
    .fetch_all(&mut **tx)
    .await?)
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
    sqlx::query("INSERT INTO convex.workspace_memberships (id,workspace_id,user_id,role) VALUES ($1,$2,$3,'owner')")
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(user)
        .execute(&mut **tx)
        .await?;
    create_team(tx, id, team_name, team_key).await?;
    Ok(id)
}

pub async fn create_team(
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
        sqlx::query(
            "INSERT INTO convex.workflow_statuses (id,workspace_id,team_id,name,category,position)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(workspace)
        .bind(id)
        .bind(name)
        .bind(category)
        .bind(position as i32)
        .execute(&mut **tx)
        .await?;
    }
    Ok(id)
}

pub async fn membership(
    tx: &mut Transaction<'_, Postgres>,
    workspace: Uuid,
    user: Uuid,
) -> AppResult<(Uuid, String)> {
    sqlx::query("SELECT id FROM convex.workspaces WHERE id=$1 FOR UPDATE")
        .bind(workspace)
        .execute(&mut **tx)
        .await?;
    sqlx::query_as("SELECT id,role FROM convex.workspace_memberships WHERE workspace_id=$1 AND user_id=$2 AND removed_at IS NULL")
        .bind(workspace)
        .bind(user)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(AppError::not_found)
}

pub async fn protect_last_owner(
    tx: &mut Transaction<'_, Postgres>,
    workspace: Uuid,
    user: Uuid,
) -> AppResult<()> {
    let owner: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1
             FROM convex.workspace_memberships
             WHERE workspace_id=$1
             AND user_id=$2
             AND role= 'owner'
             AND removed_at IS NULL)",
    )
    .bind(workspace)
    .bind(user)
    .fetch_one(&mut **tx)
    .await?;
    if owner {
        let other: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1
             FROM convex.workspace_memberships m
             JOIN convex.users u ON m.user_id=u.id
             WHERE m.workspace_id=$1
             AND m.user_id<>$2
             AND m.role= 'owner'
             AND m.removed_at IS NULL
             AND u.disabled_at IS NULL)",
        )
        .bind(workspace)
        .bind(user)
        .fetch_one(&mut **tx)
        .await?;
        if !other {
            return Err(AppError::conflict(
                "Keep at least one active workspace owner.",
            ));
        }
    }
    Ok(())
}

pub async fn valid_invite(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
) -> AppResult<ValidInvite> {
    let id:  Uuid  =  sqlx::query_scalar("SELECT id FROM convex.invitations WHERE token_hash=$1")
        .bind(digest(token))
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(||AppError::validation("This invitation is invalid, expired, or already used. Ask a workspace admin for a new link."))?;
    valid_invite_id(tx, id).await
}

pub async fn accept_for_user(
    tx: &mut Transaction<'_, Postgres>,
    invitation: &ValidInvite,
    user: Uuid,
) -> AppResult<()> {
    // Existing active membership is not demoted/promoted by accepting another invitation.
    let current: Option<bool> = sqlx::query_scalar("SELECT removed_at IS NULL FROM convex.workspace_memberships WHERE workspace_id=$1 AND user_id=$2")
        .bind(invitation.workspace_id)
        .bind(user)
        .fetch_optional(&mut **tx)
        .await?;
    if current == Some(true) {
        return Err(AppError::conflict("You already belong to this workspace."));
    }
    sqlx::query(
        "INSERT INTO convex.workspace_memberships (id,workspace_id,user_id,role)
             VALUES ($1,$2,$3,$4)
             ON CONFLICT (workspace_id,user_id) DO UPDATE
             SET removed_at=NULL,role=EXCLUDED.role,joined_at=now()",
    )
    .bind(Uuid::new_v4())
    .bind(invitation.workspace_id)
    .bind(user)
    .bind(&invitation.role)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE convex.invitations SET accepted_at=now(),accepted_by=$2 WHERE id=$1")
        .bind(invitation.id)
        .bind(user)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn valid_invite_id(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> AppResult<ValidInvite> {
    sqlx::query_as("SELECT i.id,i.workspace_id,i.role,w.name AS workspace_name,w.slug AS workspace_slug
             FROM convex.invitations i
             JOIN convex.workspaces w ON w.id=i.workspace_id
             JOIN convex.workspace_memberships m ON m.id=i.inviter_membership_id
             JOIN convex.users u ON u.id=m.user_id
             WHERE i.id=$1
             AND i.accepted_at IS NULL
             AND i.revoked_at IS NULL
             AND i.expires_at>now()
             AND m.removed_at IS NULL
             AND u.disabled_at IS NULL
             AND (m.role= 'owner'
             OR (m.role= 'admin'
             AND i.role= 'member' ))
             FOR UPDATE OF i,w,m,u")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(||AppError::validation("This invitation is invalid, expired, or already used. Ask a workspace admin for a new link."))
}

fn digest(token: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

pub use crate::validation::{manage, role, workspace_id};
