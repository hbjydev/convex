use std::sync::Arc;

use convex_issues::{
    Issue, IssueEvent, IssueEventKind, IssuePatch, NewIssue,
    repository::{IssueFilter, IssueLock},
};
use uuid::Uuid;

use crate::{ApplicationError, UnitOfWork, UnitOfWorkFactory};

#[cfg(test)]
mod tests;

type Result<T> = std::result::Result<T, ApplicationError>;

/// Events are available only after a successful commit. They currently describe
/// persisted activity; reliable external delivery would require an outbox.
pub struct OperationResult<T> {
    pub value: T,
    pub events: Vec<IssueEvent>,
}

impl<T> OperationResult<T> {
    fn without_events(value: T) -> Self {
        Self {
            value,
            events: Vec::new(),
        }
    }
}

pub enum UpdateIssueResult {
    Updated(Issue),
    Conflict(Issue),
}

#[derive(Clone)]
pub struct IssueApplication {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl IssueApplication {
    pub fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn create(
        &self,
        workspace: Uuid,
        token_digest: Option<&str>,
        mut request: NewIssue,
    ) -> Result<OperationResult<Issue>> {
        request.validate()?;
        let mut work = self.unit_of_work.begin().await?;
        let result = async {
            let actor = authorize(&mut *work, workspace, token_digest).await?;
            let status = match request.status_id {
                Some(status) => status,
                None => work
                    .workspaces()
                    .default_issue_status(workspace, request.team_id)
                    .await?
                    .ok_or(ApplicationError::validation(
                        "Choose a team with an unstarted status.",
                    ))?,
            };
            validate_references(
                &mut *work,
                workspace,
                request.team_id,
                status,
                request.assignee_membership_id,
            )
            .await?;
            request.status_id = Some(status);
            let issue = work
                .issues()
                .create(workspace, actor, &request)
                .await?
                .ok_or(ApplicationError::validation(
                    "Choose a team in this workspace.",
                ))?;
            let event =
                record_change(&mut *work, &issue, actor, IssueEventKind::Created, None).await?;
            Ok(OperationResult {
                value: issue,
                events: vec![event],
            })
        }
        .await;
        finish(work, result).await
    }

    pub async fn list(
        &self,
        workspace: Uuid,
        token_digest: Option<&str>,
        filter: IssueFilter,
    ) -> Result<OperationResult<Vec<Issue>>> {
        let mut work = self.unit_of_work.begin().await?;
        let result = async {
            authorize(&mut *work, workspace, token_digest).await?;
            if let Some(team) = filter.team_id
                && !work.workspaces().team_exists(workspace, team).await?
            {
                return Err(ApplicationError::NotFound);
            }
            let issues = work.issues().list(workspace, filter).await?;
            Ok(OperationResult::without_events(issues))
        }
        .await;
        finish(work, result).await
    }

    pub async fn detail(
        &self,
        workspace: Uuid,
        token_digest: Option<&str>,
        reference: &str,
    ) -> Result<OperationResult<Issue>> {
        let mut work = self.unit_of_work.begin().await?;
        let result = async {
            authorize(&mut *work, workspace, token_digest).await?;
            let issue = work
                .issues()
                .find(workspace, reference, IssueLock::Read)
                .await?
                .ok_or(ApplicationError::NotFound)?;
            Ok(OperationResult::without_events(issue))
        }
        .await;
        finish(work, result).await
    }

    pub async fn update(
        &self,
        workspace: Uuid,
        token_digest: Option<&str>,
        reference: &str,
        patch: IssuePatch,
    ) -> Result<OperationResult<UpdateIssueResult>> {
        if patch.expected_version < 1 {
            return Err(ApplicationError::validation(
                "Supply a positive expected_version.",
            ));
        }
        let mut work = self.unit_of_work.begin().await?;
        let result = async {
            let actor = authorize(&mut *work, workspace, token_digest).await?;
            let old = work
                .issues()
                .find(workspace, reference, IssueLock::ForUpdate)
                .await?
                .ok_or(ApplicationError::NotFound)?;
            if old.version != patch.expected_version {
                return Ok(OperationResult::without_events(
                    UpdateIssueResult::Conflict(old),
                ));
            }

            let change = old.apply(&patch)?;
            // Validate only requested references, keeping historical assignees readable.
            if patch.status_id.is_some() || patch.assignee_membership_id.is_some() {
                validate_references(
                    &mut *work,
                    workspace,
                    old.team_id,
                    patch.status_id.unwrap_or(old.status_id),
                    patch.assignee_membership_id.flatten(),
                )
                .await?;
            }
            let Some(change) = change else {
                return Ok(OperationResult::without_events(UpdateIssueResult::Updated(
                    old,
                )));
            };

            let before = work.issues().snapshot(&old).await?;
            let issue = work.issues().save(&change, actor).await?;
            let event = record_change(&mut *work, &issue, actor, change.kind, Some(before)).await?;
            Ok(OperationResult {
                value: UpdateIssueResult::Updated(issue),
                events: vec![event],
            })
        }
        .await;
        finish(work, result).await
    }
}

async fn authorize(
    work: &mut dyn UnitOfWork,
    workspace: Uuid,
    token_digest: Option<&str>,
) -> Result<Uuid> {
    // Lock before reading authentication and membership. Administration holds the
    // workspace exclusively; issue operations avoid its account/session write locks.
    work.workspaces()
        .lock_for_issue_operation(workspace)
        .await?;
    let user = work
        .sessions()
        .active_user(token_digest.ok_or(ApplicationError::Unauthorized)?)
        .await?
        .ok_or(ApplicationError::Unauthorized)?;
    work.workspaces()
        .active_membership(workspace, user)
        .await?
        .ok_or(ApplicationError::NotFound)
}

async fn validate_references(
    work: &mut dyn UnitOfWork,
    workspace: Uuid,
    team: Uuid,
    status: Uuid,
    assignee: Option<Uuid>,
) -> Result<()> {
    let repository = work.workspaces();
    if !repository
        .status_belongs_to_team(workspace, team, status)
        .await?
    {
        return Err(ApplicationError::validation(
            "Choose a status from this issue's team.",
        ));
    }
    if let Some(member) = assignee
        && !repository.can_assign_member(workspace, member).await?
    {
        return Err(ApplicationError::validation(
            "Choose an active member of this workspace as assignee.",
        ));
    }
    Ok(())
}

async fn record_change(
    work: &mut dyn UnitOfWork,
    issue: &Issue,
    actor: Uuid,
    kind: IssueEventKind,
    before: Option<convex_issues::IssueSnapshot>,
) -> Result<IssueEvent> {
    let repository = work.issues();
    let event = IssueEvent {
        workspace_id: issue.workspace_id,
        issue_id: issue.id,
        actor_membership_id: actor,
        kind,
        before,
        after: repository.snapshot(issue).await?,
    };
    repository.record_event(&event).await?;
    Ok(event)
}

async fn finish<T>(work: Box<dyn UnitOfWork>, result: Result<T>) -> Result<T> {
    match result {
        Ok(value) => {
            work.commit().await?;
            Ok(value)
        }
        Err(error) => {
            if let Err(rollback_error) = work.rollback().await {
                tracing::error!(%rollback_error, "Issue transaction rollback failed");
            }
            Err(error)
        }
    }
}
