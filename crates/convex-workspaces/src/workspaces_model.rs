use chrono::{DateTime, Utc};
use convex_ids::{InvitationId, MembershipId, TeamId, UserId, WorkflowStatusId, WorkspaceId};

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub slug: String,
    pub role: String,
}

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct Member {
    pub id: MembershipId,
    pub user_id: UserId,
    pub display_name: String,
    pub role: String,
    pub disabled: bool,
}

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct WorkflowStatus {
    pub id: WorkflowStatusId,
    pub name: String,
    pub category: String,
    pub position: i32,
}

#[derive(Clone)]
pub struct Team {
    pub id: TeamId,
    pub name: String,
    pub key: String,
    pub statuses: Vec<WorkflowStatus>,
}

#[derive(Clone)]
pub struct WorkspaceDetail {
    pub workspace: Workspace,
    pub members: Vec<Member>,
    pub teams: Vec<Team>,
}

#[derive(Clone)]
pub struct CreateWorkspace {
    pub name: String,
    pub slug: String,
    pub team_name: String,
    pub team_key: String,
}

#[derive(Clone)]
pub struct TeamRequest {
    pub name: String,
    pub key: String,
}

#[derive(Clone)]
pub struct RenameRequest {
    pub name: String,
}

#[derive(Clone)]
pub struct DeleteWorkspace {
    pub code: String,
    pub acknowledged: bool,
}

pub struct WorkspaceDeletionCode {
    pub email: String,
}

#[derive(Clone)]
pub struct MembershipChange {
    pub membership_id: MembershipId,
    pub role: Option<String>,
    pub remove: bool,
}

#[derive(Clone)]
pub struct InviteRequest {
    pub role: String,
    pub deliver_to: Option<String>,
}

#[derive(Clone)]
pub struct InviteLink {
    pub id: InvitationId,
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub email_delivered: bool,
}

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct Invitation {
    pub id: InvitationId,
    pub role: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct RevokeInvitation {
    pub invitation_id: InvitationId,
}

#[derive(Clone)]
pub struct InviteToken {
    pub token: String,
}

#[derive(Clone)]
pub struct InvitePreview {
    pub workspace_name: String,
    pub role: String,
}

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct ValidInvite {
    pub id: InvitationId,
    pub workspace_id: WorkspaceId,
    pub role: String,
    pub workspace_name: String,
    pub workspace_slug: String,
}
