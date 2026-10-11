//! Endpoint tests run the production router, middleware and application logic.
//! Only persistence and the installation policy lease are replaced with mocks.
use convex_ids::{IssueId, MembershipId, TeamId, UserId, WorkflowStatusId, WorkspaceId};
use std::sync::Arc;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use chrono::{TimeZone, Utc};
use convex_app::{
    IssueApplication,
    mock::{MockUnitOfWork, MockUnitOfWorkFactory},
};
use convex_auth::mock::MockAuthenticationPolicy;
use convex_issues::{
    Issue, IssueEventKind, IssueSnapshot, Priority,
    events::{AssigneeSnapshot, StatusSnapshot},
    repository::IssueLock,
};
use mockall::{Sequence, predicate::eq};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{http, security};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

struct Fixture {
    user: UserId,
    member: MembershipId,
    issue: Issue,
}

impl Fixture {
    fn new() -> Self {
        let created_at = Utc.with_ymd_and_hms(2026, 10, 10, 12, 0, 0).unwrap();
        Self {
            user: UserId::from_uuid(Uuid::from_u128(1)),
            member: MembershipId::from_uuid(Uuid::from_u128(2)),
            issue: Issue {
                id: IssueId::from_uuid(Uuid::from_u128(3)),
                workspace_id: WorkspaceId::from_uuid(Uuid::from_u128(4)),
                team_id: TeamId::from_uuid(Uuid::from_u128(5)),
                number: 42,
                identifier: "ENG-42".into(),
                title: "Original title".into(),
                description: "Issue description".into(),
                status_id: WorkflowStatusId::from_uuid(Uuid::from_u128(6)),
                priority: Priority::None,
                assignee_membership_id: None,
                creator_membership_id: MembershipId::from_uuid(Uuid::from_u128(2)),
                version: 1,
                created_at,
                updated_at: created_at,
                archived_at: None,
                archived_by_membership_id: None,
            },
        }
    }

    fn collection(&self) -> String {
        format!("/api/v1/workspaces/{}/issues", self.issue.workspace_id)
    }

    fn detail(&self) -> String {
        format!("{}/ENG-42", self.collection())
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

fn authorized_work(fixture: &Fixture, sequence: &mut Sequence) -> MockUnitOfWork {
    let mut work = MockUnitOfWork::default();
    work.workspaces
        .expect_lock_for_issue_operation()
        .with(eq(fixture.issue.workspace_id))
        .times(1)
        .in_sequence(sequence)
        .returning(|_| Ok(()));
    let user = fixture.user;
    work.sessions
        .expect_active_user()
        .with(eq(security::digest(TOKEN)))
        .times(1)
        .in_sequence(sequence)
        .returning(move |_| Ok(Some(user)));
    let member = fixture.member;
    work.workspaces
        .expect_active_membership()
        .with(eq(fixture.issue.workspace_id), eq(fixture.user))
        .times(1)
        .in_sequence(sequence)
        .returning(move |_, _| Ok(Some(member)));
    work
}

fn expect_find(work: &mut MockUnitOfWork, issue: &Issue, lock: IssueLock) {
    let loaded = issue.clone();
    work.issues
        .expect_find()
        .with(eq(issue.workspace_id), eq("ENG-42"), eq(lock))
        .times(1)
        .returning(move |_, _, _| Ok(Some(loaded.clone())));
}

fn expect_commit(work: &mut MockUnitOfWork, sequence: &mut Sequence) {
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(sequence)
        .returning(|| Ok(()));
}

fn expect_rollback(work: &mut MockUnitOfWork) {
    work.lifecycle
        .expect_rollback()
        .times(1)
        .returning(|| Ok(()));
}

async fn router(factory: MockUnitOfWorkFactory, policy: MockAuthenticationPolicy) -> Router {
    let mut dependencies = crate::test_support::services();
    dependencies.issues = IssueApplication::new(Arc::new(factory));
    dependencies.authentication_policy = Arc::new(policy);
    http::router(
        crate::test_support::cookies("http://127.0.0.1:5173"),
        dependencies,
    )
}

async fn app(work: Option<MockUnitOfWork>) -> Router {
    let mut factory = MockUnitOfWorkFactory::new();
    if let Some(work) = work {
        factory
            .expect_begin()
            .times(1)
            .return_once(move || Ok(Box::new(work)));
    }
    let mut policy = MockAuthenticationPolicy::new();
    policy.expect_check().times(1).returning(|| Ok(()));
    router(factory, policy).await
}

fn request(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("origin", "http://127.0.0.1:5173")
        .header("x-convex-csrf", "1")
        .header("content-type", "application/json")
        .header("cookie", format!("convex_session={TOKEN}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn call(app: Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app.oneshot(request(method, path, body)).await.unwrap();
    let status = response.status();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    assert_eq!(response.headers()["content-type"], "application/json");
    let body =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, body)
}

#[tokio::test]
async fn editing_records_history_before_commit_and_returns_the_saved_issue() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    expect_find(&mut work, &fixture.issue, IssueLock::ForUpdate);
    let before = snapshot(&fixture.issue);
    work.issues
        .expect_snapshot()
        .withf(|issue| issue.version == 1)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move |_| Ok(before.clone()));
    let mut saved = fixture.issue.clone();
    saved.title = "Edited title".into();
    saved.version = 2;
    let saved_result = saved.clone();
    work.issues
        .expect_save()
        .withf(move |change, actor| {
            change.issue.title == "Edited title"
                && change.issue.version == 1
                && change.kind == IssueEventKind::Updated
                && *actor == MembershipId::from_uuid(Uuid::from_u128(2))
        })
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move |_, _| Ok(saved_result.clone()));
    let after = snapshot(&saved);
    work.issues
        .expect_snapshot()
        .withf(|issue| issue.version == 2)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move |_| Ok(after.clone()));
    work.issues
        .expect_record_event()
        .withf(|event| {
            event.kind == IssueEventKind::Updated
                && event.before.as_ref().unwrap().version == 1
                && event.after.version == 2
                && event.after.title == "Edited title"
        })
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    expect_commit(&mut work, &mut sequence);

    let (status, body) = call(
        app(Some(work)).await,
        "PATCH",
        &fixture.detail(),
        json!({"expected_version":1,"title":"  Edited title  "}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "Edited title");
    assert_eq!(body["version"], 2);
    assert_eq!(body["identifier"], "ENG-42");
}

#[tokio::test]
async fn stale_edit_returns_current_issue_without_writing_or_recording_activity() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    expect_find(&mut work, &fixture.issue, IssueLock::ForUpdate);
    expect_commit(&mut work, &mut sequence);
    let (status, body) = call(
        app(Some(work)).await,
        "PATCH",
        &fixture.detail(),
        json!({"expected_version":2,"title":"Draft"}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "conflict");
    assert_eq!(body["current_issue"]["title"], "Original title");
    assert_eq!(body["current_issue"]["version"], 1);
}

#[tokio::test]
async fn unchanged_edit_keeps_the_version_and_produces_no_activity() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    expect_find(&mut work, &fixture.issue, IssueLock::ForUpdate);
    expect_commit(&mut work, &mut sequence);
    let (status, body) = call(
        app(Some(work)).await,
        "PATCH",
        &fixture.detail(),
        json!({"expected_version":1,"title":" Original title "}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], 1);
}

#[tokio::test]
async fn activity_failure_rolls_back_and_hides_internal_details() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    expect_find(&mut work, &fixture.issue, IssueLock::ForUpdate);
    work.issues
        .expect_snapshot()
        .times(2)
        .returning(|issue| Ok(snapshot(issue)));
    work.issues.expect_save().times(1).returning(|change, _| {
        let mut saved = change.issue.clone();
        saved.version += 1;
        Ok(saved)
    });
    work.issues
        .expect_record_event()
        .times(1)
        .returning(|_| Err(anyhow::anyhow!("private database details")));
    expect_rollback(&mut work);
    let (status, body) = call(
        app(Some(work)).await,
        "PATCH",
        &fixture.detail(),
        json!({"expected_version":1,"title":"Draft"}),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "service_unavailable");
    assert!(!body.to_string().contains("private database details"));
}

#[tokio::test]
async fn commit_failure_does_not_report_a_successful_edit() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    expect_find(&mut work, &fixture.issue, IssueLock::ForUpdate);
    work.lifecycle
        .expect_commit()
        .times(1)
        .returning(|| Err(anyhow::anyhow!("commit failed")));
    let (status, body) = call(
        app(Some(work)).await,
        "PATCH",
        &fixture.detail(),
        json!({"expected_version":1}),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "service_unavailable");
}

#[tokio::test]
async fn missing_membership_returns_not_found_before_loading_issue() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = MockUnitOfWork::default();
    work.workspaces
        .expect_lock_for_issue_operation()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.sessions
        .expect_active_user()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(Some(UserId::from_uuid(Uuid::from_u128(1)))));
    work.workspaces
        .expect_active_membership()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok(None));
    expect_rollback(&mut work);
    let (status, body) = call(app(Some(work)).await, "GET", &fixture.detail(), Value::Null).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn expired_session_returns_unauthorized_before_reading_membership() {
    let fixture = Fixture::new();
    let mut work = MockUnitOfWork::default();
    work.workspaces
        .expect_lock_for_issue_operation()
        .times(1)
        .returning(|_| Ok(()));
    work.sessions
        .expect_active_user()
        .times(1)
        .returning(|_| Ok(None));
    expect_rollback(&mut work);
    let (status, body) = call(app(Some(work)).await, "GET", &fixture.detail(), Value::Null).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "unauthorized");
}

#[tokio::test]
async fn detail_returns_an_archived_issue() {
    let mut fixture = Fixture::new();
    fixture.issue.archived_at = Some(fixture.issue.created_at);
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    expect_find(&mut work, &fixture.issue, IssueLock::Read);
    expect_commit(&mut work, &mut sequence);
    let (status, body) = call(app(Some(work)).await, "GET", &fixture.detail(), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["archived_at"].is_string());
}

#[tokio::test]
async fn list_passes_archive_and_team_filters_to_the_repository() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    let team = fixture.issue.team_id;
    work.workspaces
        .expect_team_exists()
        .with(eq(fixture.issue.workspace_id), eq(team))
        .times(1)
        .returning(|_, _| Ok(true));
    let issue = fixture.issue.clone();
    work.issues
        .expect_list()
        .withf(move |_, filter| filter.archived && filter.team_id == Some(team))
        .times(1)
        .returning(move |_, _| Ok(vec![issue.clone()]));
    expect_commit(&mut work, &mut sequence);
    let path = format!("{}?archived=true&team_id={team}", fixture.collection());
    let (status, body) = call(app(Some(work)).await, "GET", &path, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["identifier"], "ENG-42");
}

#[tokio::test]
async fn a_foreign_team_returns_not_found_without_listing_issues() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    work.workspaces
        .expect_team_exists()
        .times(1)
        .returning(|_, _| Ok(false));
    expect_rollback(&mut work);
    let path = format!(
        "{}?team_id={}",
        fixture.collection(),
        TeamId::from_uuid(Uuid::from_u128(99))
    );
    let (status, body) = call(app(Some(work)).await, "GET", &path, Value::Null).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn create_resolves_the_default_status_and_records_creation_before_commit() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    let status_id = fixture.issue.status_id;
    work.workspaces
        .expect_default_issue_status()
        .with(eq(fixture.issue.workspace_id), eq(fixture.issue.team_id))
        .times(1)
        .returning(move |_, _| Ok(Some(status_id)));
    work.workspaces
        .expect_status_belongs_to_team()
        .with(
            eq(fixture.issue.workspace_id),
            eq(fixture.issue.team_id),
            eq(status_id),
        )
        .times(1)
        .returning(|_, _, _| Ok(true));
    let issue = fixture.issue.clone();
    work.issues
        .expect_create()
        .withf(move |_, _, issue| {
            issue.title == "Original title" && issue.status_id == Some(status_id)
        })
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move |_, _, _| Ok(Some(issue.clone())));
    work.issues
        .expect_snapshot()
        .times(1)
        .returning(|issue| Ok(snapshot(issue)));
    work.issues
        .expect_record_event()
        .withf(|event| event.kind == IssueEventKind::Created && event.before.is_none())
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    expect_commit(&mut work, &mut sequence);
    let (status, body) = call(
        app(Some(work)).await,
        "POST",
        &fixture.collection(),
        json!({"team_id":fixture.issue.team_id,"title":" Original title "}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["priority"], "none");
    assert_eq!(body["status_id"], status_id.to_string());
}

#[tokio::test]
async fn archived_issue_rejects_property_edits_without_writing() {
    let mut fixture = Fixture::new();
    fixture.issue.archived_at = Some(fixture.issue.created_at);
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    expect_find(&mut work, &fixture.issue, IssueLock::ForUpdate);
    expect_rollback(&mut work);
    let (status, body) = call(
        app(Some(work)).await,
        "PATCH",
        &fixture.detail(),
        json!({"expected_version":1,"title":"Draft"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "Restore this issue before editing it.");
}

#[tokio::test]
async fn archive_and_property_edits_cannot_be_combined() {
    let fixture = Fixture::new();
    let mut sequence = Sequence::new();
    let mut work = authorized_work(&fixture, &mut sequence);
    expect_find(&mut work, &fixture.issue, IssueLock::ForUpdate);
    expect_rollback(&mut work);
    let (status, body) = call(
        app(Some(work)).await,
        "PATCH",
        &fixture.detail(),
        json!({"expected_version":1,"archived":true,"title":"Draft"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["message"],
        "Archive or restore separately from property edits."
    );
}

#[tokio::test]
async fn null_clears_an_assignee_while_omission_preserves_it() {
    for clears_assignee in [false, true] {
        let mut fixture = Fixture::new();
        fixture.issue.assignee_membership_id = Some(MembershipId::from_uuid(Uuid::from_u128(9)));
        let mut sequence = Sequence::new();
        let mut work = authorized_work(&fixture, &mut sequence);
        expect_find(&mut work, &fixture.issue, IssueLock::ForUpdate);
        if clears_assignee {
            work.workspaces
                .expect_status_belongs_to_team()
                .times(1)
                .returning(|_, _, _| Ok(true));
        }
        work.issues
            .expect_snapshot()
            .times(2)
            .returning(|issue| Ok(snapshot(issue)));
        work.issues
            .expect_save()
            .withf(move |change, _| {
                change.issue.assignee_membership_id
                    == if clears_assignee {
                        None
                    } else {
                        Some(MembershipId::from_uuid(Uuid::from_u128(9)))
                    }
            })
            .times(1)
            .returning(|change, _| {
                let mut issue = change.issue.clone();
                issue.version += 1;
                Ok(issue)
            });
        work.issues
            .expect_record_event()
            .times(1)
            .returning(|_| Ok(()));
        expect_commit(&mut work, &mut sequence);
        let mut patch = json!({"expected_version":1,"title":"Edited"});
        if clears_assignee {
            patch["assignee_membership_id"] = Value::Null;
        }
        let (status, body) = call(app(Some(work)).await, "PATCH", &fixture.detail(), patch).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["assignee_membership_id"].is_null(), clears_assignee);
    }
}

#[tokio::test]
async fn invalid_patch_shapes_are_rejected_before_beginning_a_transaction() {
    let fixture = Fixture::new();
    for patch in [
        json!({"expected_version":1,"title":null}),
        json!({"expected_version":1,"priority":null}),
        json!({"expected_version":1,"archived":null}),
        json!({"expected_version":1,"unknown":true}),
        json!({"expected_version":0}),
    ] {
        let (status, body) = call(app(None).await, "PATCH", &fixture.detail(), patch).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "validation");
    }
}

#[tokio::test]
async fn policy_failure_prevents_any_repository_access() {
    let fixture = Fixture::new();
    let mut policy = MockAuthenticationPolicy::new();
    policy
        .expect_check()
        .times(1)
        .returning(|| Err(anyhow::anyhow!("lease lost")));
    let app = router(MockUnitOfWorkFactory::new(), policy).await;
    let (status, body) = call(app, "GET", &fixture.detail(), Value::Null).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "service_unavailable");
}

#[tokio::test]
async fn csrf_failure_prevents_policy_and_repository_access() {
    let fixture = Fixture::new();
    let app = router(
        MockUnitOfWorkFactory::new(),
        MockAuthenticationPolicy::new(),
    )
    .await;
    let mut request = request("PATCH", &fixture.detail(), json!({"expected_version":1}));
    request.headers_mut().remove("x-convex-csrf");
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn invalid_issue_input_is_rejected_before_authentication_even_without_a_cookie() {
    let fixture = Fixture::new();
    for (method, path, body) in [
        (
            "POST",
            fixture.collection(),
            json!({"team_id":fixture.issue.team_id,"title":" "}),
        ),
        ("PATCH", fixture.detail(), json!({"expected_version":0})),
    ] {
        let mut request = request(method, &path, body);
        request.headers_mut().remove("cookie");
        let response = app(None).await.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn missing_cookie_rolls_back_without_reading_a_session() {
    let fixture = Fixture::new();
    let mut work = MockUnitOfWork::default();
    work.workspaces
        .expect_lock_for_issue_operation()
        .times(1)
        .returning(|_| Ok(()));
    expect_rollback(&mut work);
    let mut request = request("GET", &fixture.detail(), Value::Null);
    request.headers_mut().remove("cookie");
    let response = app(Some(work)).await.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
