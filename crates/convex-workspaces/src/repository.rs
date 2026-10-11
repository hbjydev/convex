use anyhow::Result;
use async_trait::async_trait;
use convex_ids::{InvitationId, MembershipId, TeamId, UserId, WorkflowStatusId, WorkspaceId};

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait WorkspaceRepository: Send {
    async fn set_deletion_code(
        &mut self,
        workspace: WorkspaceId,
        user: UserId,
        code_hash: String,
    ) -> convex_core::Result<()>;
    async fn consume_deletion_code(
        &mut self,
        workspace: WorkspaceId,
        user: UserId,
        code_hash: String,
    ) -> convex_core::Result<bool>;
    async fn delete(&mut self, workspace: WorkspaceId) -> convex_core::Result<()>;
    async fn list_for_user(&mut self, user: UserId) -> convex_core::Result<Vec<crate::Workspace>>;
    async fn create(
        &mut self,
        user: UserId,
        name: String,
        slug: String,
        team_name: String,
        team_key: String,
    ) -> convex_core::Result<WorkspaceId>;
    async fn identity(&mut self, workspace: WorkspaceId) -> convex_core::Result<(String, String)>;
    async fn rename(&mut self, workspace: WorkspaceId, name: String) -> convex_core::Result<()>;
    async fn lock_all(&mut self) -> convex_core::Result<Vec<WorkspaceId>>;

    /// Acquired before session or membership reads. Administration uses an
    /// exclusive workspace lock, preventing removal/disablement during an issue operation.
    async fn lock_for_issue_operation(&mut self, workspace: WorkspaceId) -> Result<()>;
    async fn active_membership(
        &mut self,
        workspace: WorkspaceId,
        user: UserId,
    ) -> Result<Option<MembershipId>>;
    async fn team_exists(&mut self, workspace: WorkspaceId, team: TeamId) -> Result<bool>;
    async fn default_issue_status(
        &mut self,
        workspace: WorkspaceId,
        team: TeamId,
    ) -> Result<Option<WorkflowStatusId>>;
    async fn status_belongs_to_team(
        &mut self,
        workspace: WorkspaceId,
        team: TeamId,
        status: WorkflowStatusId,
    ) -> Result<bool>;
    async fn can_assign_member(
        &mut self,
        workspace: WorkspaceId,
        member: MembershipId,
    ) -> Result<bool>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait MembershipRepository: Send {
    /// Locks the workspace before reading authorization state.
    async fn for_user(
        &mut self,
        workspace: WorkspaceId,
        user: UserId,
    ) -> convex_core::Result<(MembershipId, String)>;
    async fn list(&mut self, workspace: WorkspaceId) -> convex_core::Result<Vec<crate::Member>>;
    async fn target(
        &mut self,
        workspace: WorkspaceId,
        member: MembershipId,
    ) -> convex_core::Result<(UserId, String)>;
    async fn protect_last_owner(
        &mut self,
        workspace: WorkspaceId,
        user: UserId,
    ) -> convex_core::Result<()>;
    async fn remove(&mut self, member: MembershipId) -> convex_core::Result<()>;
    async fn change_role(&mut self, member: MembershipId, role: String) -> convex_core::Result<()>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait TeamRepository: Send {
    async fn create(
        &mut self,
        workspace: WorkspaceId,
        name: String,
        key: String,
    ) -> convex_core::Result<TeamId>;
    async fn list(
        &mut self,
        workspace: WorkspaceId,
    ) -> convex_core::Result<Vec<(TeamId, String, String)>>;
    async fn rename(
        &mut self,
        workspace: WorkspaceId,
        team: TeamId,
        name: String,
    ) -> convex_core::Result<()>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait WorkflowRepository: Send {
    async fn for_team(&mut self, team: TeamId) -> convex_core::Result<Vec<crate::WorkflowStatus>>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait InvitationRepository: Send {
    async fn create(
        &mut self,
        id: InvitationId,
        workspace: WorkspaceId,
        inviter: MembershipId,
        role: String,
        token_digest: String,
    ) -> convex_core::Result<chrono::DateTime<chrono::Utc>>;
    async fn list(&mut self, workspace: WorkspaceId)
    -> convex_core::Result<Vec<crate::Invitation>>;
    async fn revoke(
        &mut self,
        workspace: WorkspaceId,
        invitation: InvitationId,
    ) -> convex_core::Result<()>;
    async fn valid_token(&mut self, token: String) -> convex_core::Result<crate::ValidInvite>;
    async fn valid_id(&mut self, id: InvitationId) -> convex_core::Result<crate::ValidInvite>;
    async fn accept(
        &mut self,
        invitation: crate::ValidInvite,
        user: UserId,
    ) -> convex_core::Result<()>;
}
