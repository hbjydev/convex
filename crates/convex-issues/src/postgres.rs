use anyhow::{Result, ensure};
use async_trait::async_trait;
use convex_db::postgres::PostgresTransaction;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    Issue, IssueChange, IssueEvent, IssueEventKind, IssueSnapshot, NewIssue,
    events::{AssigneeSnapshot, StatusSnapshot},
    repository::{IssueFilter, IssueLock, IssueRepository},
};

const SELECT_ISSUE: &str = "
    SELECT i.*, t.key || '-' || i.number::text AS identifier
    FROM convex.issues i
    JOIN convex.teams t ON t.id = i.team_id";

#[async_trait]
impl IssueRepository for PostgresTransaction {
    async fn clear_assignments(
        &mut self,
        workspace: Uuid,
        member: Uuid,
        actor: Uuid,
    ) -> Result<()> {
        clear_assignments(self.connection(), workspace, member, actor).await
    }
    async fn find(
        &mut self,
        workspace: Uuid,
        reference: &str,
        lock: IssueLock,
    ) -> Result<Option<Issue>> {
        find(self.connection(), workspace, reference, lock).await
    }

    async fn list(&mut self, workspace: Uuid, filter: IssueFilter) -> Result<Vec<Issue>> {
        let query = format!(
            "{SELECT_ISSUE}
             WHERE i.workspace_id = $1
               AND (i.archived_at IS NOT NULL) = $2
               AND ($3::uuid IS NULL OR i.team_id = $3)
             ORDER BY i.created_at DESC, i.id DESC"
        );
        Ok(sqlx::query_as(&query)
            .bind(workspace)
            .bind(filter.archived)
            .bind(filter.team_id)
            .fetch_all(self.connection())
            .await?)
    }

    async fn create(
        &mut self,
        workspace: Uuid,
        actor: Uuid,
        issue: &NewIssue,
    ) -> Result<Option<Issue>> {
        let number: Option<i64> = sqlx::query_scalar(
            "UPDATE convex.teams SET next_issue_number = next_issue_number + 1
             WHERE workspace_id = $1 AND id = $2
             RETURNING next_issue_number - 1",
        )
        .bind(workspace)
        .bind(issue.team_id)
        .fetch_optional(self.connection())
        .await?;
        let Some(number) = number else {
            return Ok(None);
        };

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO convex.issues (
                 id, workspace_id, team_id, number, title, description,
                 status_id, priority, assignee_membership_id, creator_membership_id
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(workspace)
        .bind(issue.team_id)
        .bind(number)
        .bind(&issue.title)
        .bind(&issue.description)
        .bind(issue.status_id)
        .bind(issue.priority)
        .bind(issue.assignee_membership_id)
        .bind(actor)
        .execute(self.connection())
        .await?;
        find(
            self.connection(),
            workspace,
            &id.to_string(),
            IssueLock::Read,
        )
        .await
    }

    async fn save(&mut self, change: &IssueChange, actor: Uuid) -> Result<Issue> {
        let issue = &change.issue;
        let updated = sqlx::query(
            "UPDATE convex.issues SET
                 title = $3, description = $4, status_id = $5, priority = $6,
                 assignee_membership_id = $7, version = version + 1, updated_at = now(),
                 archived_at = CASE WHEN $8 THEN coalesce(archived_at, now()) ELSE NULL END,
                 archived_by_membership_id = CASE
                     WHEN $8 THEN coalesce(archived_by_membership_id, $9) ELSE NULL END
             WHERE workspace_id = $1 AND id = $2 AND version = $10",
        )
        .bind(issue.workspace_id)
        .bind(issue.id)
        .bind(&issue.title)
        .bind(&issue.description)
        .bind(issue.status_id)
        .bind(issue.priority)
        .bind(issue.assignee_membership_id)
        .bind(change.archived)
        .bind(actor)
        .bind(issue.version)
        .execute(self.connection())
        .await?
        .rows_affected();
        ensure!(
            updated == 1,
            "locked issue version changed before persistence"
        );
        find(
            self.connection(),
            issue.workspace_id,
            &issue.id.to_string(),
            IssueLock::Read,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("saved issue could not be loaded"))
    }

    async fn snapshot(&mut self, issue: &Issue) -> Result<IssueSnapshot> {
        snapshot(self.connection(), issue).await
    }

    async fn record_event(&mut self, event: &IssueEvent) -> Result<()> {
        record_event(self.connection(), event).await
    }
}

async fn find(
    connection: &mut PgConnection,
    workspace: Uuid,
    reference: &str,
    lock: IssueLock,
) -> Result<Option<Issue>> {
    let lock = match lock {
        IssueLock::Read => "",
        IssueLock::ForUpdate => " FOR UPDATE OF i",
    };
    let query = format!(
        "{SELECT_ISSUE}
         WHERE i.workspace_id = $1
           AND (i.id::text = $2 OR t.key || '-' || i.number::text = $2){lock}"
    );
    Ok(sqlx::query_as(&query)
        .bind(workspace)
        .bind(reference)
        .fetch_optional(connection)
        .await?)
}

async fn snapshot(connection: &mut PgConnection, issue: &Issue) -> Result<IssueSnapshot> {
    let status_name = sqlx::query_scalar("SELECT name FROM convex.workflow_statuses WHERE id = $1")
        .bind(issue.status_id)
        .fetch_one(&mut *connection)
        .await?;
    let assignee_name = sqlx::query_scalar(
        "SELECT u.display_name FROM convex.workspace_memberships m
         JOIN convex.users u ON u.id = m.user_id WHERE m.id = $1",
    )
    .bind(issue.assignee_membership_id)
    .fetch_optional(&mut *connection)
    .await?;
    Ok(IssueSnapshot {
        title: issue.title.clone(),
        description: issue.description.clone(),
        status: StatusSnapshot {
            id: issue.status_id,
            name: status_name,
        },
        priority: issue.priority,
        assignee: AssigneeSnapshot {
            id: issue.assignee_membership_id,
            name: assignee_name,
        },
        archived_at: issue.archived_at,
        version: issue.version,
    })
}

async fn record_event(connection: &mut PgConnection, event: &IssueEvent) -> Result<()> {
    let actor_name: String = sqlx::query_scalar(
        "SELECT u.display_name FROM convex.workspace_memberships m
         JOIN convex.users u ON u.id = m.user_id WHERE m.id = $1",
    )
    .bind(event.actor_membership_id)
    .fetch_one(&mut *connection)
    .await?;
    let payload = json!({ "actor_name": actor_name, "before": event.before, "after": event.after });
    sqlx::query(
        "INSERT INTO convex.issue_activity (
             id, workspace_id, issue_id, actor_membership_id, kind, payload
         ) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(event.workspace_id)
    .bind(event.issue_id)
    .bind(event.actor_membership_id)
    .bind(event.kind.as_str())
    .bind(payload)
    .execute(connection)
    .await?;
    Ok(())
}

/// Bridge for membership administration during the staged extraction. The caller
/// owns the transaction and holds the workspace exclusively. Includes archived
/// issues, increments each changed version, and records activity in that transaction.
pub async fn clear_assignments(
    connection: &mut PgConnection,
    workspace: Uuid,
    member: Uuid,
    actor: Uuid,
) -> Result<()> {
    let query = format!(
        "{SELECT_ISSUE}
         WHERE i.workspace_id = $1 AND i.assignee_membership_id = $2
         ORDER BY i.id FOR UPDATE OF i"
    );
    let assigned: Vec<Issue> = sqlx::query_as(&query)
        .bind(workspace)
        .bind(member)
        .fetch_all(&mut *connection)
        .await?;
    for old in assigned {
        let before = snapshot(connection, &old).await?;
        sqlx::query(
            "UPDATE convex.issues SET assignee_membership_id = NULL,
             version = version + 1, updated_at = now() WHERE id = $1",
        )
        .bind(old.id)
        .execute(&mut *connection)
        .await?;
        let issue = find(connection, workspace, &old.id.to_string(), IssueLock::Read)
            .await?
            .ok_or_else(|| anyhow::anyhow!("unassigned issue could not be loaded"))?;
        let after = snapshot(connection, &issue).await?;
        record_event(
            connection,
            &IssueEvent {
                workspace_id: workspace,
                issue_id: issue.id,
                actor_membership_id: actor,
                kind: IssueEventKind::Updated,
                before: Some(before),
                after,
            },
        )
        .await?;
    }
    Ok(())
}
