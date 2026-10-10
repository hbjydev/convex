use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::events::IssueEventKind;

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "postgres", derive(sqlx::Type))]
#[cfg_attr(
    feature = "postgres",
    sqlx(type_name = "text", rename_all = "snake_case")
)]
pub enum Priority {
    #[default]
    None,
    Urgent,
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
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

#[derive(Clone, Debug)]
pub struct NewIssue {
    pub team_id: Uuid,
    pub title: String,
    pub description: String,
    pub status_id: Option<Uuid>,
    pub priority: Priority,
    pub assignee_membership_id: Option<Uuid>,
}

impl NewIssue {
    pub fn validate(&mut self) -> Result<(), IssueError> {
        self.title = valid_title(&self.title)?;
        valid_description(&self.description)
    }
}

/// A domain command. The HTTP layer preserves omitted versus explicitly cleared
/// assignees when constructing it; no serialization rules live here.
#[derive(Clone, Debug, Default)]
pub struct IssuePatch {
    pub expected_version: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status_id: Option<Uuid>,
    pub priority: Option<Priority>,
    pub assignee_membership_id: Option<Option<Uuid>>,
    pub archived: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct IssueError(pub &'static str);

/// The intended mutation, before database timestamps and the new version are applied.
#[derive(Clone, Debug)]
pub struct IssueChange {
    pub issue: Issue,
    pub archived: bool,
    pub kind: IssueEventKind,
}

impl Issue {
    /// Returns None for a no-op. The application checks expected_version first,
    /// then validates any changed references before persisting the change.
    pub fn apply(&self, patch: &IssuePatch) -> Result<Option<IssueChange>, IssueError> {
        let edits_properties = patch.title.is_some()
            || patch.description.is_some()
            || patch.status_id.is_some()
            || patch.priority.is_some()
            || patch.assignee_membership_id.is_some();

        if patch.archived.is_some() && edits_properties {
            return Err(IssueError(
                "Archive or restore separately from property edits.",
            ));
        }
        if self.archived_at.is_some() && edits_properties {
            return Err(IssueError("Restore this issue before editing it."));
        }

        let mut next = self.clone();
        if let Some(title) = &patch.title {
            next.title = valid_title(title)?;
        }
        if let Some(description) = &patch.description {
            valid_description(description)?;
            next.description.clone_from(description);
        }
        if let Some(status) = patch.status_id {
            next.status_id = status;
        }
        if let Some(priority) = patch.priority {
            next.priority = priority;
        }
        if let Some(assignee) = patch.assignee_membership_id {
            next.assignee_membership_id = assignee;
        }

        let was_archived = self.archived_at.is_some();
        let archived = patch.archived.unwrap_or(was_archived);
        let changed = next.title != self.title
            || next.description != self.description
            || next.status_id != self.status_id
            || next.priority != self.priority
            || next.assignee_membership_id != self.assignee_membership_id
            || archived != was_archived;
        if !changed {
            return Ok(None);
        }

        let kind = match (was_archived, archived) {
            (false, true) => IssueEventKind::Archived,
            (true, false) => IssueEventKind::Restored,
            _ => IssueEventKind::Updated,
        };
        Ok(Some(IssueChange {
            issue: next,
            archived,
            kind,
        }))
    }
}

fn valid_title(value: &str) -> Result<String, IssueError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 240 {
        return Err(IssueError("Enter an issue title of 1–240 characters."));
    }
    Ok(value.into())
}

fn valid_description(value: &str) -> Result<(), IssueError> {
    if value.chars().count() > 50_000 {
        return Err(IssueError(
            "Descriptions can contain at most 50,000 characters.",
        ));
    }
    Ok(())
}

impl From<IssueError> for convex_core::Error {
    fn from(error: IssueError) -> Self {
        Self::validation(error.0)
    }
}
