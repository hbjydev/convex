use chrono::{DateTime, Utc};
use convex_ids::{IssueId, MembershipId, WorkflowStatusId, WorkspaceId};
use serde::Serialize;

use crate::Priority;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueEventKind {
    Created,
    Updated,
    Archived,
    Restored,
}

impl IssueEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Archived => "archived",
            Self::Restored => "restored",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusSnapshot {
    pub id: WorkflowStatusId,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AssigneeSnapshot {
    pub id: Option<MembershipId>,
    pub name: Option<String>,
}

/// Names are captured inside the mutation's transaction, so activity remains
/// readable after a status or account is renamed.
#[derive(Clone, Debug, Serialize)]
pub struct IssueSnapshot {
    pub title: String,
    pub description: String,
    pub status: StatusSnapshot,
    pub priority: Priority,
    pub assignee: AssigneeSnapshot,
    pub archived_at: Option<DateTime<Utc>>,
    pub version: i64,
}

/// A fact produced by an issue mutation. Activity is persisted before commit;
/// events are returned to callers only after commit. There is no external delivery yet.
#[derive(Clone, Debug)]
pub struct IssueEvent {
    pub workspace_id: WorkspaceId,
    pub issue_id: IssueId,
    pub actor_membership_id: MembershipId,
    pub kind: IssueEventKind,
    pub before: Option<IssueSnapshot>,
    pub after: IssueSnapshot,
}
