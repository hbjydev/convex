//! Issue models and rules, independent of HTTP and database access.

pub mod events;
pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres;
pub mod repository;
#[cfg(feature = "mock")]
pub mod mock {
    pub use crate::repository::MockIssueRepository;
}

pub use events::{IssueEvent, IssueEventKind, IssueSnapshot};
pub use model::{Issue, IssueChange, IssueError, IssuePatch, NewIssue, Priority};
