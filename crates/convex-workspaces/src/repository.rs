use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait WorkspaceRepository: Send {
    async fn list_for_user(&mut self, user: Uuid) -> convex_core::Result<Vec<crate::Workspace>>;
    async fn create(
        &mut self,
        user: Uuid,
        name: String,
        slug: String,
        team_name: String,
        team_key: String,
    ) -> convex_core::Result<Uuid>;
    async fn identity(&mut self, workspace: Uuid) -> convex_core::Result<(String, String)>;
    async fn rename(&mut self, workspace: Uuid, name: String) -> convex_core::Result<()>;
    async fn lock_all(&mut self) -> convex_core::Result<Vec<Uuid>>;

    /// Acquired before session or membership reads. Administration uses an
    /// exclusive workspace lock, preventing removal/disablement during an issue operation.
    async fn lock_for_issue_operation(&mut self, workspace: Uuid) -> Result<()>;
    async fn active_membership(&mut self, workspace: Uuid, user: Uuid) -> Result<Option<Uuid>>;
    async fn team_exists(&mut self, workspace: Uuid, team: Uuid) -> Result<bool>;
    async fn default_issue_status(&mut self, workspace: Uuid, team: Uuid) -> Result<Option<Uuid>>;
    async fn status_belongs_to_team(
        &mut self,
        workspace: Uuid,
        team: Uuid,
        status: Uuid,
    ) -> Result<bool>;
    async fn can_assign_member(&mut self, workspace: Uuid, member: Uuid) -> Result<bool>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait MembershipRepository: Send {
    /// Locks the workspace before reading authorization state.
    async fn for_user(
        &mut self,
        workspace: Uuid,
        user: Uuid,
    ) -> convex_core::Result<(Uuid, String)>;
    async fn list(&mut self, workspace: Uuid) -> convex_core::Result<Vec<crate::Member>>;
    async fn target(
        &mut self,
        workspace: Uuid,
        member: Uuid,
    ) -> convex_core::Result<(Uuid, String)>;
    async fn protect_last_owner(&mut self, workspace: Uuid, user: Uuid) -> convex_core::Result<()>;
    async fn remove(&mut self, member: Uuid) -> convex_core::Result<()>;
    async fn change_role(&mut self, member: Uuid, role: String) -> convex_core::Result<()>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait TeamRepository: Send {
    async fn create(
        &mut self,
        workspace: Uuid,
        name: String,
        key: String,
    ) -> convex_core::Result<Uuid>;
    async fn list(&mut self, workspace: Uuid) -> convex_core::Result<Vec<(Uuid, String, String)>>;
    async fn rename(
        &mut self,
        workspace: Uuid,
        team: Uuid,
        name: String,
    ) -> convex_core::Result<()>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait WorkflowRepository: Send {
    async fn for_team(&mut self, team: Uuid) -> convex_core::Result<Vec<crate::WorkflowStatus>>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait InvitationRepository: Send {
    async fn create(
        &mut self,
        id: Uuid,
        workspace: Uuid,
        inviter: Uuid,
        role: String,
        token_digest: String,
    ) -> convex_core::Result<chrono::DateTime<chrono::Utc>>;
    async fn list(&mut self, workspace: Uuid) -> convex_core::Result<Vec<crate::Invitation>>;
    async fn revoke(&mut self, workspace: Uuid, invitation: Uuid) -> convex_core::Result<()>;
    async fn valid_token(&mut self, token: String) -> convex_core::Result<crate::ValidInvite>;
    async fn valid_id(&mut self, id: Uuid) -> convex_core::Result<crate::ValidInvite>;
    async fn accept(
        &mut self,
        invitation: crate::ValidInvite,
        user: Uuid,
    ) -> convex_core::Result<()>;
}
