use convex_ids::{IssueId, WorkflowStatusId};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use convex_core::Error;
use convex_issues::{
    Issue, IssueEventKind, IssuePatch, IssueSnapshot, NewIssue, Priority,
    events::{AssigneeSnapshot, StatusSnapshot},
    repository::{IssueFilter, IssueLock},
};
use mockall::{Sequence, predicate::eq};
use uuid::Uuid;

use super::{IssueApplication, UpdateIssueResult};
use crate::{
    mock::{MockUnitOfWork, MockUnitOfWorkFactory},
    test_support::*,
};

const DIGEST: &str = "session-digest";
const STATUS: WorkflowStatusId = WorkflowStatusId::from_uuid(Uuid::from_u128(7));

fn issue() -> Issue {
    let created_at = Utc.with_ymd_and_hms(2026, 10, 10, 12, 0, 0).unwrap();
    Issue {
        id: IssueId::from_uuid(Uuid::from_u128(8)),
        workspace_id: WORKSPACE,
        team_id: TEAM,
        number: 42,
        identifier: "ENG-42".into(),
        title: "Original title".into(),
        description: "Issue description".into(),
        status_id: STATUS,
        priority: Priority::None,
        assignee_membership_id: None,
        creator_membership_id: ACTOR_MEMBER,
        version: 1,
        created_at,
        updated_at: created_at,
        archived_at: None,
        archived_by_membership_id: None,
    }
}

fn snapshot(issue: &Issue) -> IssueSnapshot {
    IssueSnapshot {
        title: issue.title.clone(),
        description: issue.description.clone(),
        status: StatusSnapshot {
            id: issue.status_id,
            name: "Todo".into(),
        },
        priority: issue.priority,
        assignee: AssigneeSnapshot {
            id: issue.assignee_membership_id,
            name: None,
        },
        archived_at: issue.archived_at,
        version: issue.version,
    }
}

fn application(work: Option<MockUnitOfWork>) -> IssueApplication {
    let mut factory = MockUnitOfWorkFactory::new();
    if let Some(work) = work {
        factory
            .expect_begin()
            .times(1)
            .return_once(move || Ok(Box::new(work)));
    }
    IssueApplication::new(Arc::new(factory))
}

fn authorized_work(sequence: &mut Sequence) -> MockUnitOfWork {
    let mut work = MockUnitOfWork::default();
    work.workspaces
        .expect_lock_for_issue_operation()
        .with(eq(WORKSPACE))
        .times(1)
        .in_sequence(sequence)
        .returning(|_| Ok(()));
    work.sessions
        .expect_active_user()
        .with(eq(DIGEST))
        .times(1)
        .in_sequence(sequence)
        .returning(|_| Ok(Some(USER)));
    work.workspaces
        .expect_active_membership()
        .with(eq(WORKSPACE), eq(USER))
        .times(1)
        .in_sequence(sequence)
        .returning(|_, _| Ok(Some(ACTOR_MEMBER)));
    work
}

fn expect_find(work: &mut MockUnitOfWork, current: Issue) {
    work.issues
        .expect_find()
        .with(eq(WORKSPACE), eq("ENG-42"), eq(IssueLock::ForUpdate))
        .times(1)
        .return_once(move |_, _, _| Ok(Some(current)));
}

fn new_issue() -> NewIssue {
    NewIssue {
        team_id: TEAM,
        title: " Original title ".into(),
        description: "Issue description".into(),
        status_id: None,
        priority: Priority::None,
        assignee_membership_id: None,
    }
}

#[tokio::test]
async fn creation_returns_the_committed_event_with_its_actor_and_snapshot() {
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&mut sequence);
    work.workspaces
        .expect_default_issue_status()
        .with(eq(WORKSPACE), eq(TEAM))
        .times(1)
        .returning(|_, _| Ok(Some(STATUS)));
    work.workspaces
        .expect_status_belongs_to_team()
        .with(eq(WORKSPACE), eq(TEAM), eq(STATUS))
        .times(1)
        .returning(|_, _, _| Ok(true));
    work.issues
        .expect_create()
        .withf(|workspace, actor, request| {
            *workspace == WORKSPACE
                && *actor == ACTOR_MEMBER
                && request.title == "Original title"
                && request.status_id == Some(STATUS)
        })
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _, _| Ok(Some(issue())));
    work.issues
        .expect_snapshot()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|issue| Ok(snapshot(issue)));
    work.issues
        .expect_record_event()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));

    let result = application(Some(work))
        .create(WORKSPACE, Some(DIGEST), new_issue())
        .await
        .unwrap();

    assert_eq!(result.value.identifier, "ENG-42");
    assert_eq!(result.events.len(), 1);
    let event = &result.events[0];
    assert_eq!(event.workspace_id, WORKSPACE);
    assert_eq!(event.issue_id, result.value.id);
    assert_eq!(event.actor_membership_id, ACTOR_MEMBER);
    assert_eq!(event.kind, IssueEventKind::Created);
    assert!(event.before.is_none());
    assert_eq!(event.after.title, result.value.title);
    assert_eq!(event.after.status.name, "Todo");
    assert_eq!(event.after.version, result.value.version);
}

#[tokio::test]
async fn version_conflicts_return_the_current_issue_without_events_or_writes() {
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&mut sequence);
    let mut current = issue();
    current.version = 3;
    expect_find(&mut work, current);
    expect_commit(&mut work);
    let result = application(Some(work))
        .update(
            WORKSPACE,
            Some(DIGEST),
            "ENG-42",
            IssuePatch {
                expected_version: 1,
                title: Some("Edited".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(result.events.is_empty());
    assert!(
        matches!(result.value, UpdateIssueResult::Conflict(issue) if issue.version == 3 && issue.title == "Original title")
    );
}

#[tokio::test]
async fn no_op_edits_return_the_unchanged_version_without_events_or_writes() {
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&mut sequence);
    expect_find(&mut work, issue());
    expect_commit(&mut work);
    let result = application(Some(work))
        .update(
            WORKSPACE,
            Some(DIGEST),
            "ENG-42",
            IssuePatch {
                expected_version: 1,
                title: Some(" Original title ".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(result.events.is_empty());
    assert!(matches!(result.value, UpdateIssueResult::Updated(issue) if issue.version == 1));
}

#[tokio::test]
async fn archive_and_restore_events_use_database_timestamps_and_versions() {
    for archived in [true, false] {
        let mut sequence = Sequence::new();
        let mut work = authorized_work(&mut sequence);
        let mut current = issue();
        let archived_at = current.created_at + chrono::Duration::hours(1);
        if !archived {
            current.archived_at = Some(archived_at);
            current.archived_by_membership_id = Some(ACTOR_MEMBER);
        }
        expect_find(&mut work, current);
        work.issues
            .expect_snapshot()
            .withf(|issue| issue.version == 1)
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|issue| Ok(snapshot(issue)));
        work.issues
            .expect_save()
            .withf(move |change, actor| change.archived == archived && *actor == ACTOR_MEMBER)
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |change, _| {
                let mut saved = change.issue.clone();
                saved.version = 2;
                saved.updated_at = archived_at;
                saved.archived_at = archived.then_some(archived_at);
                saved.archived_by_membership_id = archived.then_some(ACTOR_MEMBER);
                Ok(saved)
            });
        work.issues
            .expect_snapshot()
            .withf(|issue| issue.version == 2)
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|issue| Ok(snapshot(issue)));
        work.issues
            .expect_record_event()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_| Ok(()));
        work.lifecycle
            .expect_commit()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|| Ok(()));
        let result = application(Some(work))
            .update(
                WORKSPACE,
                Some(DIGEST),
                "ENG-42",
                IssuePatch {
                    expected_version: 1,
                    archived: Some(archived),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let event = &result.events[0];
        assert_eq!(
            event.kind,
            if archived {
                IssueEventKind::Archived
            } else {
                IssueEventKind::Restored
            }
        );
        let before = event.before.as_ref().unwrap();
        assert_eq!(before.version, 1);
        assert_eq!(before.archived_at, (!archived).then_some(archived_at));
        assert_eq!(event.after.version, 2);
        assert_eq!(event.after.archived_at, archived.then_some(archived_at));
    }
}

#[tokio::test]
async fn activity_failure_rolls_back_the_saved_issue_and_returns_no_operation_result() {
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&mut sequence);
    expect_find(&mut work, issue());
    work.issues
        .expect_snapshot()
        .times(2)
        .returning(|issue| Ok(snapshot(issue)));
    work.issues
        .expect_save()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|change, _| {
            let mut saved = change.issue.clone();
            saved.version += 1;
            Ok(saved)
        });
    work.issues
        .expect_record_event()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Err(anyhow::anyhow!("activity failed")));
    work.lifecycle
        .expect_rollback()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let result = application(Some(work))
        .update(
            WORKSPACE,
            Some(DIGEST),
            "ENG-42",
            IssuePatch {
                expected_version: 1,
                title: Some("Edited".into()),
                ..Default::default()
            },
        )
        .await;
    assert!(
        matches!(result, Err(Error::Infrastructure(error)) if error.to_string() == "activity failed")
    );
}

#[tokio::test]
async fn commit_failure_does_not_publish_recorded_events() {
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&mut sequence);
    expect_find(&mut work, issue());
    work.issues
        .expect_snapshot()
        .times(2)
        .returning(|issue| Ok(snapshot(issue)));
    work.issues
        .expect_save()
        .times(1)
        .returning(|change, _| Ok(change.issue.clone()));
    work.issues
        .expect_record_event()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Err(anyhow::anyhow!("commit failed")));
    let result = application(Some(work))
        .update(
            WORKSPACE,
            Some(DIGEST),
            "ENG-42",
            IssuePatch {
                expected_version: 1,
                title: Some("Edited".into()),
                ..Default::default()
            },
        )
        .await;
    assert!(
        matches!(result, Err(Error::Infrastructure(error)) if error.to_string() == "commit failed")
    );
}

#[tokio::test]
async fn foreign_statuses_and_inactive_assignees_are_rejected_before_writing() {
    for status_valid in [false, true] {
        let mut sequence = Sequence::new();
        let mut work = authorized_work(&mut sequence);
        expect_find(&mut work, issue());
        work.workspaces
            .expect_status_belongs_to_team()
            .with(eq(WORKSPACE), eq(TEAM), eq(STATUS))
            .times(1)
            .returning(move |_, _, _| Ok(status_valid));
        if status_valid {
            work.workspaces
                .expect_can_assign_member()
                .with(eq(WORKSPACE), eq(TARGET_MEMBER))
                .times(1)
                .returning(|_, _| Ok(false));
        }
        expect_rollback(&mut work);
        let result = application(Some(work))
            .update(
                WORKSPACE,
                Some(DIGEST),
                "ENG-42",
                IssuePatch {
                    expected_version: 1,
                    assignee_membership_id: Some(Some(TARGET_MEMBER)),
                    ..Default::default()
                },
            )
            .await;
        assert!(
            matches!(result, Err(Error::Validation(message)) if message == if status_valid {
            "Choose an active member of this workspace as assignee."
        } else { "Choose a status from this issue's team." })
        );
    }
}

#[tokio::test]
async fn missing_default_status_rolls_back_creation_without_inserting_an_issue() {
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&mut sequence);
    work.workspaces
        .expect_default_issue_status()
        .with(eq(WORKSPACE), eq(TEAM))
        .times(1)
        .returning(|_, _| Ok(None));
    expect_rollback(&mut work);
    let result = application(Some(work))
        .create(WORKSPACE, Some(DIGEST), new_issue())
        .await;
    assert!(
        matches!(result, Err(Error::Validation(message)) if message == "Choose a team with an unstarted status.")
    );
}

#[tokio::test]
async fn invalid_commands_do_not_begin_a_transaction() {
    let mut request = new_issue();
    request.title = " ".into();
    assert!(matches!(
        application(None)
            .create(WORKSPACE, Some(DIGEST), request)
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        application(None)
            .update(WORKSPACE, Some(DIGEST), "ENG-42", IssuePatch::default())
            .await,
        Err(Error::Validation(_))
    ));
}

#[tokio::test]
async fn expired_sessions_are_rejected_under_the_workspace_lock_without_reading_issues() {
    let mut work = MockUnitOfWork::default();
    let mut sequence = Sequence::new();
    work.workspaces
        .expect_lock_for_issue_operation()
        .with(eq(WORKSPACE))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.sessions
        .expect_active_user()
        .with(eq(DIGEST))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(None));
    work.lifecycle
        .expect_rollback()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let result = application(Some(work))
        .detail(WORKSPACE, Some(DIGEST), "ENG-42")
        .await;
    assert!(matches!(result, Err(Error::Unauthorized)));
}

#[tokio::test]
async fn members_of_other_workspaces_cannot_list_issues() {
    let mut work = MockUnitOfWork::default();
    work.workspaces
        .expect_lock_for_issue_operation()
        .with(eq(WORKSPACE))
        .times(1)
        .returning(|_| Ok(()));
    work.sessions
        .expect_active_user()
        .with(eq(DIGEST))
        .times(1)
        .returning(|_| Ok(Some(USER)));
    work.workspaces
        .expect_active_membership()
        .with(eq(WORKSPACE), eq(USER))
        .times(1)
        .returning(|_, _| Ok(None));
    expect_rollback(&mut work);
    let result = application(Some(work))
        .list(WORKSPACE, Some(DIGEST), IssueFilter::default())
        .await;
    assert!(matches!(result, Err(Error::NotFound)));
}

#[tokio::test]
async fn read_results_have_no_events() {
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&mut sequence);
    work.issues
        .expect_find()
        .with(eq(WORKSPACE), eq("ENG-42"), eq(IssueLock::Read))
        .times(1)
        .returning(|_, _, _| Ok(Some(issue())));
    expect_commit(&mut work);
    let result = application(Some(work))
        .detail(WORKSPACE, Some(DIGEST), "ENG-42")
        .await
        .unwrap();
    assert_eq!(result.value.id, issue().id);
    assert!(result.events.is_empty());
}

#[tokio::test]
async fn edits_return_the_same_before_and_after_snapshots_that_were_recorded() {
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&mut sequence);
    expect_find(&mut work, issue());
    work.issues
        .expect_snapshot()
        .withf(|issue| issue.version == 1)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|issue| {
            let mut before = snapshot(issue);
            before.status.name = "Previous status name".into();
            Ok(before)
        });
    work.issues
        .expect_save()
        .withf(|change, actor| change.issue.title == "Edited" && *actor == ACTOR_MEMBER)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|change, _| {
            let mut saved = change.issue.clone();
            saved.version = 2;
            Ok(saved)
        });
    work.issues
        .expect_snapshot()
        .withf(|issue| issue.version == 2)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|issue| Ok(snapshot(issue)));
    let recorded = Arc::new(std::sync::Mutex::new(None));
    let captured = recorded.clone();
    work.issues
        .expect_record_event()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move |event| {
            *captured.lock().unwrap() = Some(event.clone());
            Ok(())
        });
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let result = application(Some(work))
        .update(
            WORKSPACE,
            Some(DIGEST),
            "ENG-42",
            IssuePatch {
                expected_version: 1,
                title: Some("Edited".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(result.events.len(), 1);
    let event = &result.events[0];
    assert_eq!(event.kind, IssueEventKind::Updated);
    let before = event.before.as_ref().unwrap();
    assert_eq!(before.title, "Original title");
    assert_eq!(before.status.name, "Previous status name");
    assert_eq!(event.after.title, "Edited");
    assert_eq!(event.after.version, 2);
    let recorded = recorded.lock().unwrap();
    let recorded = recorded.as_ref().unwrap();
    assert_eq!(
        event.before.as_ref().unwrap().status.name,
        recorded.before.as_ref().unwrap().status.name
    );
    assert_eq!(event.after.title, recorded.after.title);
    assert_eq!(event.after.version, recorded.after.version);
}
