use crate::accounts::models::AcceptInvite;
use async_trait::async_trait;
use convex_auth::accounts::models::MessageResponse;
use convex_auth::credentials::{Credentials, SessionUpdate};
use convex_core::Result;
use convex_workspaces::workspaces_model::*;

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait Workspaces: Send + Sync {
    async fn create(&self, credentials: Credentials, req: CreateWorkspace) -> Result<Workspace>;
    async fn detail(&self, credentials: Credentials, space: String) -> Result<WorkspaceDetail>;
    async fn rename(
        &self,
        credentials: Credentials,
        space: String,
        req: RenameRequest,
    ) -> Result<MessageResponse>;
    async fn team(
        &self,
        credentials: Credentials,
        space: String,
        req: TeamRequest,
    ) -> Result<MessageResponse>;
    async fn rename_team(
        &self,
        credentials: Credentials,
        space: String,
        team: String,
        req: RenameRequest,
    ) -> Result<MessageResponse>;
    async fn change_member(
        &self,
        credentials: Credentials,
        space: String,
        req: MembershipChange,
    ) -> Result<MessageResponse>;
    async fn invite(
        &self,
        credentials: Credentials,
        space: String,
        req: InviteRequest,
    ) -> Result<InviteLink>;
    async fn invitations(&self, credentials: Credentials, space: String)
    -> Result<Vec<Invitation>>;
    async fn revoke_invitation(
        &self,
        credentials: Credentials,
        space: String,
        req: RevokeInvitation,
    ) -> Result<MessageResponse>;
    async fn preview(&self, req: InviteToken) -> Result<InvitePreview>;
    async fn accept(
        &self,
        credentials: Credentials,
        req: AcceptInvite,
    ) -> Result<SessionUpdate<Workspace>>;
}

#[async_trait]
impl Workspaces for crate::control_plane::ControlPlane {
    async fn create(&self, credentials: Credentials, req: CreateWorkspace) -> Result<Workspace> {
        crate::workspace_workflows::create(self.clone(), credentials, req).await
    }
    async fn detail(&self, credentials: Credentials, space: String) -> Result<WorkspaceDetail> {
        crate::workspace_workflows::detail(self.clone(), credentials, space).await
    }
    async fn rename(
        &self,
        credentials: Credentials,
        space: String,
        req: RenameRequest,
    ) -> Result<MessageResponse> {
        crate::workspace_workflows::rename(self.clone(), credentials, space, req).await
    }
    async fn team(
        &self,
        credentials: Credentials,
        space: String,
        req: TeamRequest,
    ) -> Result<MessageResponse> {
        crate::workspace_workflows::team(self.clone(), credentials, space, req).await
    }
    async fn rename_team(
        &self,
        credentials: Credentials,
        space: String,
        team: String,
        req: RenameRequest,
    ) -> Result<MessageResponse> {
        crate::workspace_workflows::rename_team(self.clone(), credentials, space, team, req).await
    }
    async fn change_member(
        &self,
        credentials: Credentials,
        space: String,
        req: MembershipChange,
    ) -> Result<MessageResponse> {
        crate::workspace_workflows::change_member(self.clone(), credentials, space, req).await
    }
    async fn invite(
        &self,
        credentials: Credentials,
        space: String,
        req: InviteRequest,
    ) -> Result<InviteLink> {
        crate::workspace_workflows::invite(self.clone(), credentials, space, req).await
    }
    async fn invitations(
        &self,
        credentials: Credentials,
        space: String,
    ) -> Result<Vec<Invitation>> {
        crate::workspace_workflows::invitations(self.clone(), credentials, space).await
    }
    async fn revoke_invitation(
        &self,
        credentials: Credentials,
        space: String,
        req: RevokeInvitation,
    ) -> Result<MessageResponse> {
        crate::workspace_workflows::revoke_invitation(self.clone(), credentials, space, req).await
    }
    async fn preview(&self, req: InviteToken) -> Result<InvitePreview> {
        crate::workspace_workflows::preview(self.clone(), req).await
    }
    async fn accept(
        &self,
        credentials: Credentials,
        req: AcceptInvite,
    ) -> Result<SessionUpdate<Workspace>> {
        crate::workspace_workflows::accept(self.clone(), credentials, req).await
    }
}
