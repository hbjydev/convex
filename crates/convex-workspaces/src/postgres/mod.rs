use anyhow::Result;
use async_trait::async_trait;
use convex_db::postgres::PostgresTransaction;
use uuid::Uuid;

use crate::repository::WorkspaceRepository;

#[async_trait]
impl WorkspaceRepository for PostgresTransaction {
    async fn list_for_user(&mut self, user: Uuid) -> convex_core::Result<Vec<crate::Workspace>> {
        administration::list_for_user(self.transaction(), user).await
    }
    async fn create(
        &mut self,
        user: Uuid,
        name: String,
        slug: String,
        team_name: String,
        team_key: String,
    ) -> convex_core::Result<Uuid> {
        administration::create_workspace(self.transaction(), user, name, slug, team_name, team_key)
            .await
    }
    async fn identity(&mut self, workspace: Uuid) -> convex_core::Result<(String, String)> {
        Ok(
            sqlx::query_as("SELECT name,slug FROM convex.workspaces WHERE id=$1")
                .bind(workspace)
                .fetch_one(self.connection())
                .await?,
        )
    }
    async fn rename(&mut self, workspace: Uuid, name: String) -> convex_core::Result<()> {
        sqlx::query("UPDATE convex.workspaces SET name=$2,updated_at=now() WHERE id=$1")
            .bind(workspace)
            .bind(name)
            .execute(self.connection())
            .await?;
        Ok(())
    }
    async fn lock_all(&mut self) -> convex_core::Result<Vec<Uuid>> {
        Ok(
            sqlx::query_scalar("SELECT id FROM convex.workspaces ORDER BY id FOR UPDATE")
                .fetch_all(self.connection())
                .await?,
        )
    }

    async fn lock_for_issue_operation(&mut self, workspace: Uuid) -> Result<()> {
        sqlx::query("SELECT id FROM convex.workspaces WHERE id = $1 FOR SHARE")
            .bind(workspace)
            .execute(self.connection())
            .await?;
        Ok(())
    }

    async fn active_membership(&mut self, workspace: Uuid, user: Uuid) -> Result<Option<Uuid>> {
        Ok(sqlx::query_scalar(
            "SELECT id FROM convex.workspace_memberships
             WHERE workspace_id = $1 AND user_id = $2 AND removed_at IS NULL",
        )
        .bind(workspace)
        .bind(user)
        .fetch_optional(self.connection())
        .await?)
    }

    async fn team_exists(&mut self, workspace: Uuid, team: Uuid) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM convex.teams WHERE workspace_id = $1 AND id = $2)",
        )
        .bind(workspace)
        .bind(team)
        .fetch_one(self.connection())
        .await?)
    }

    async fn default_issue_status(&mut self, workspace: Uuid, team: Uuid) -> Result<Option<Uuid>> {
        Ok(sqlx::query_scalar(
            "SELECT id FROM convex.workflow_statuses
             WHERE workspace_id = $1 AND team_id = $2 AND category = 'unstarted'
             ORDER BY position, id LIMIT 1",
        )
        .bind(workspace)
        .bind(team)
        .fetch_optional(self.connection())
        .await?)
    }

    async fn status_belongs_to_team(
        &mut self,
        workspace: Uuid,
        team: Uuid,
        status: Uuid,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM convex.workflow_statuses
             WHERE workspace_id = $1 AND team_id = $2 AND id = $3)",
        )
        .bind(workspace)
        .bind(team)
        .bind(status)
        .fetch_one(self.connection())
        .await?)
    }

    async fn can_assign_member(&mut self, workspace: Uuid, member: Uuid) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(
                 SELECT 1 FROM convex.workspace_memberships m
                 JOIN convex.users u ON u.id = m.user_id
                 WHERE m.workspace_id = $1 AND m.id = $2
                   AND m.removed_at IS NULL AND u.disabled_at IS NULL
             )",
        )
        .bind(workspace)
        .bind(member)
        .fetch_one(self.connection())
        .await?)
    }
}

pub mod administration;

#[async_trait]
impl crate::repository::MembershipRepository for PostgresTransaction {
    async fn for_user(
        &mut self,
        workspace: Uuid,
        user: Uuid,
    ) -> convex_core::Result<(Uuid, String)> {
        administration::membership(self.transaction(), workspace, user).await
    }
    async fn list(&mut self, workspace: Uuid) -> convex_core::Result<Vec<crate::Member>> {
        Ok(sqlx::query_as(
            "SELECT m.id,m.user_id,u.display_name,m.role,u.disabled_at IS NOT NULL AS disabled
             FROM convex.workspace_memberships m
             JOIN convex.users u ON m.user_id=u.id
             WHERE m.workspace_id=$1
             AND m.removed_at IS NULL
             ORDER BY m.joined_at,m.id",
        )
        .bind(workspace)
        .fetch_all(self.connection())
        .await?)
    }
    async fn target(
        &mut self,
        workspace: Uuid,
        member: Uuid,
    ) -> convex_core::Result<(Uuid, String)> {
        sqlx::query_as("SELECT user_id,role FROM convex.workspace_memberships WHERE id=$1 AND workspace_id=$2 AND removed_at IS NULL")
        .bind(member)
        .bind(workspace)
        .fetch_optional(self.connection())
        .await?
        .ok_or_else(convex_core::Error::not_found)
    }
    async fn protect_last_owner(&mut self, workspace: Uuid, user: Uuid) -> convex_core::Result<()> {
        administration::protect_last_owner(self.transaction(), workspace, user).await
    }
    async fn remove(&mut self, member: Uuid) -> convex_core::Result<()> {
        sqlx::query("UPDATE convex.workspace_memberships SET removed_at=now() WHERE id=$1")
            .bind(member)
            .execute(self.connection())
            .await?;
        Ok(())
    }
    async fn change_role(&mut self, member: Uuid, role: String) -> convex_core::Result<()> {
        sqlx::query("UPDATE convex.workspace_memberships SET role=$2 WHERE id=$1")
            .bind(member)
            .bind(role)
            .execute(self.connection())
            .await?;
        Ok(())
    }
}
#[async_trait]
impl crate::repository::TeamRepository for PostgresTransaction {
    async fn create(
        &mut self,
        workspace: Uuid,
        name: String,
        key: String,
    ) -> convex_core::Result<Uuid> {
        administration::create_team(self.transaction(), workspace, name, key).await
    }
    async fn list(&mut self, workspace: Uuid) -> convex_core::Result<Vec<(Uuid, String, String)>> {
        Ok(sqlx::query_as(
            "SELECT id,name,key FROM convex.teams WHERE workspace_id=$1 ORDER BY created_at,id",
        )
        .bind(workspace)
        .fetch_all(self.connection())
        .await?)
    }
    async fn rename(
        &mut self,
        workspace: Uuid,
        team: Uuid,
        name: String,
    ) -> convex_core::Result<()> {
        if sqlx::query(
            "UPDATE convex.teams SET name=$3,updated_at=now() WHERE id=$2 AND workspace_id=$1",
        )
        .bind(workspace)
        .bind(team)
        .bind(name)
        .execute(self.connection())
        .await?
        .rows_affected()
            == 0
        {
            return Err(convex_core::Error::not_found());
        }
        Ok(())
    }
}
#[async_trait]
impl crate::repository::WorkflowRepository for PostgresTransaction {
    async fn for_team(&mut self, team: Uuid) -> convex_core::Result<Vec<crate::WorkflowStatus>> {
        Ok(sqlx::query_as("SELECT id,name,category,position FROM convex.workflow_statuses WHERE team_id=$1 ORDER BY position,id")
        .bind(team)
        .fetch_all(self.connection())
        .await?)
    }
}
#[async_trait]
impl crate::repository::InvitationRepository for PostgresTransaction {
    async fn create(
        &mut self,
        id: Uuid,
        workspace: Uuid,
        inviter: Uuid,
        role: String,
        token_digest: String,
    ) -> convex_core::Result<chrono::DateTime<chrono::Utc>> {
        Ok(sqlx::query_scalar(
            "INSERT INTO convex.invitations (id,workspace_id,inviter_membership_id,role,token_hash)
             VALUES ($1,$2,$3,$4,$5)
             RETURNING expires_at",
        )
        .bind(id)
        .bind(workspace)
        .bind(inviter)
        .bind(role)
        .bind(token_digest)
        .fetch_one(self.connection())
        .await?)
    }
    async fn list(&mut self, workspace: Uuid) -> convex_core::Result<Vec<crate::Invitation>> {
        Ok(sqlx::query_as(
            "SELECT id,role,expires_at,accepted_at,revoked_at
             FROM convex.invitations
             WHERE workspace_id=$1
             ORDER BY created_at DESC,id",
        )
        .bind(workspace)
        .fetch_all(self.connection())
        .await?)
    }
    async fn revoke(&mut self, workspace: Uuid, invitation: Uuid) -> convex_core::Result<()> {
        if sqlx::query(
            "UPDATE convex.invitations SET revoked_at=now() WHERE id=$1 AND workspace_id=$2",
        )
        .bind(invitation)
        .bind(workspace)
        .execute(self.connection())
        .await?
        .rows_affected()
            == 0
        {
            return Err(convex_core::Error::not_found());
        }
        Ok(())
    }
    async fn valid_token(&mut self, token: String) -> convex_core::Result<crate::ValidInvite> {
        administration::valid_invite(self.transaction(), &token).await
    }
    async fn valid_id(&mut self, id: Uuid) -> convex_core::Result<crate::ValidInvite> {
        administration::valid_invite_id(self.transaction(), id).await
    }
    async fn accept(
        &mut self,
        invitation: crate::ValidInvite,
        user: Uuid,
    ) -> convex_core::Result<()> {
        administration::accept_for_user(self.transaction(), &invitation, user).await
    }
}
