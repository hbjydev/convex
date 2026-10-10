use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::{Issue, IssueChange, IssueEvent, IssueSnapshot, NewIssue};

#[derive(Clone, Copy, Debug, Default)]
pub struct IssueFilter {
    pub archived: bool,
    pub team_id: Option<Uuid>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueLock {
    Read,
    ForUpdate,
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait IssueRepository: Send {
    async fn clear_assignments(&mut self, workspace: Uuid, member: Uuid, actor: Uuid)
    -> Result<()>;
    /// Resolves a UUID or exact identifier within one workspace, including archived issues.
    /// ForUpdate holds the issue row lock until the unit of work finishes.
    async fn find(
        &mut self,
        workspace: Uuid,
        reference: &str,
        lock: IssueLock,
    ) -> Result<Option<Issue>>;
    async fn list(&mut self, workspace: Uuid, filter: IssueFilter) -> Result<Vec<Issue>>;
    /// Allocates the next team number and inserts the issue in the same transaction.
    /// None means the team does not belong to this workspace.
    async fn create(
        &mut self,
        workspace: Uuid,
        actor: Uuid,
        issue: &NewIssue,
    ) -> Result<Option<Issue>>;
    /// Caller previously loaded the issue ForUpdate. Updates exactly the loaded version,
    /// increments it once, and returns database-assigned timestamps.
    async fn save(&mut self, change: &IssueChange, actor: Uuid) -> Result<Issue>;
    async fn snapshot(&mut self, issue: &Issue) -> Result<IssueSnapshot>;
    /// Activity must share the issue mutation's transaction; failure aborts that mutation.
    async fn record_event(&mut self, event: &IssueEvent) -> Result<()>;
}
