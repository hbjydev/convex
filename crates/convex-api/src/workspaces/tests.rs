//! Exercise HTTP and cross-domain mutations together, replacing only repositories.
use std::sync::Arc;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::Response,
};
use convex_app::{
    accounts::AccountApplication,
    control_plane::ControlPlane,
    mock::{MockUnitOfWork, MockUnitOfWorkFactory},
};
use convex_auth::{
    accounts::MockAccountAuthentication,
    accounts::models::User,
    mock::{MockAuthenticationPolicy, MockRateLimiter},
    security::{LoginMethod, SecurityConfig},
};
use convex_core::Error;
use convex_workspaces::ValidInvite;
use mockall::{Sequence, predicate::eq};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SPACE: Uuid = Uuid::from_u128(1);
const ACTOR: Uuid = Uuid::from_u128(2);
const ACTOR_MEMBER: Uuid = Uuid::from_u128(3);
const TARGET: Uuid = Uuid::from_u128(4);
const TARGET_MEMBER: Uuid = Uuid::from_u128(5);

fn router(work: MockUnitOfWork, rate_limiter: MockRateLimiter) -> Router {
    let mut factory = MockUnitOfWorkFactory::new();
    factory
        .expect_begin_administration()
        .times(1)
        .return_once(move || Ok(Box::new(work)));
    let control_plane = ControlPlane {
        transactions: Arc::new(factory),
        security: SecurityConfig::development(),
        rate_limiter: Arc::new(rate_limiter),
    };
    let mut services = crate::test_support::services();
    services.accounts = Arc::new(AccountApplication {
        control_plane: control_plane.clone(),
        authentication: Arc::new(MockAccountAuthentication::new()),
    });
    services.workspaces = Arc::new(control_plane);
    let mut policy = MockAuthenticationPolicy::new();
    policy.expect_check().times(1).returning(|| Ok(()));
    services.authentication_policy = Arc::new(policy);
    crate::router(
        crate::test_support::cookies("http://127.0.0.1:5173"),
        services,
    )
}

async fn post(router: Router, path: &str, body: Value) -> Response {
    router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("origin", "http://127.0.0.1:5173")
                .header("x-convex-csrf", "1")
                .header("content-type", "application/json")
                .header("cookie", format!("convex_session={TOKEN}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body(response: Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

fn authorized(work: &mut MockUnitOfWork, sequence: &mut Sequence, role: &str) {
    work.accounts
        .expect_authenticate()
        .withf(|credentials| credentials.session.as_deref() == Some(TOKEN))
        .times(1)
        .in_sequence(sequence)
        .returning(|_| {
            Ok(User {
                id: ACTOR,
                display_name: "Workspace owner".into(),
                is_instance_admin: true,
            })
        });
    let role = role.to_owned();
    work.memberships
        .expect_for_user()
        .with(eq(SPACE), eq(ACTOR))
        .times(1)
        .in_sequence(sequence)
        .returning(move |_, _| Ok((ACTOR_MEMBER, role.clone())));
}

fn removal(work: &mut MockUnitOfWork, sequence: &mut Sequence) {
    authorized(work, sequence, "owner");
    work.memberships
        .expect_target()
        .with(eq(SPACE), eq(TARGET_MEMBER))
        .times(1)
        .in_sequence(sequence)
        .returning(|_, _| Ok((TARGET, "member".into())));
    work.memberships
        .expect_protect_last_owner()
        .with(eq(SPACE), eq(TARGET))
        .times(1)
        .in_sequence(sequence)
        .returning(|_, _| Ok(()));
}

fn expect_cleanup(work: &mut MockUnitOfWork, sequence: &mut Sequence) {
    work.issues
        .expect_clear_assignments()
        .with(eq(SPACE), eq(TARGET_MEMBER), eq(ACTOR_MEMBER))
        .times(1)
        .in_sequence(sequence)
        .returning(|_, _, _| Ok(()));
    work.memberships
        .expect_remove()
        .with(eq(TARGET_MEMBER))
        .times(1)
        .in_sequence(sequence)
        .returning(|_| Ok(()));
}

#[tokio::test]
async fn membership_removal_commits_after_issue_cleanup_and_session_revocation() {
    let mut sequence = Sequence::new();
    let mut work = MockUnitOfWork::default();
    removal(&mut work, &mut sequence);
    expect_cleanup(&mut work, &mut sequence);
    work.sessions
        .expect_revoke_all()
        .with(eq(TARGET))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let response = post(
        router(work, MockRateLimiter::new()),
        &format!("/api/v1/workspaces/{SPACE}/members"),
        json!({"membership_id":TARGET_MEMBER,"remove":true}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await["message"],
        "Membership updated. This person's sessions were revoked."
    );
}

#[tokio::test]
async fn failed_issue_cleanup_rolls_back_without_removing_membership() {
    let mut sequence = Sequence::new();
    let mut work = MockUnitOfWork::default();
    removal(&mut work, &mut sequence);
    work.issues
        .expect_clear_assignments()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _, _| Err(anyhow::anyhow!("Activity persistence failed")));
    work.lifecycle
        .expect_rollback()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let response = post(
        router(work, MockRateLimiter::new()),
        &format!("/api/v1/workspaces/{SPACE}/members"),
        json!({"membership_id":TARGET_MEMBER,"remove":true}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body(response).await["code"], "service_unavailable");
}

#[tokio::test]
async fn failed_session_revocation_rolls_back_membership_and_issue_changes() {
    let mut sequence = Sequence::new();
    let mut work = MockUnitOfWork::default();
    removal(&mut work, &mut sequence);
    expect_cleanup(&mut work, &mut sequence);
    work.sessions
        .expect_revoke_all()
        .with(eq(TARGET))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Err(Error::internal("Revocation failed")));
    work.lifecycle
        .expect_rollback()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let response = post(
        router(work, MockRateLimiter::new()),
        &format!("/api/v1/workspaces/{SPACE}/members"),
        json!({"membership_id":TARGET_MEMBER,"remove":true}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn final_owner_cannot_be_removed() {
    let mut sequence = Sequence::new();
    let mut work = MockUnitOfWork::default();
    authorized(&mut work, &mut sequence, "owner");
    work.memberships
        .expect_target()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok((TARGET, "owner".into())));
    work.memberships
        .expect_protect_last_owner()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Err(Error::conflict("Keep at least one active workspace owner.")));
    work.lifecycle
        .expect_rollback()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let response = post(
        router(work, MockRateLimiter::new()),
        &format!("/api/v1/workspaces/{SPACE}/members"),
        json!({"membership_id":TARGET_MEMBER,"remove":true}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(response).await["message"],
        "Keep at least one active workspace owner."
    );
}

#[tokio::test]
async fn team_creation_uses_workspace_role_and_transactional_repository() {
    let mut sequence = Sequence::new();
    let mut work = MockUnitOfWork::default();
    authorized(&mut work, &mut sequence, "admin");
    work.teams
        .expect_create()
        .with(
            eq(SPACE),
            eq("Engineering".to_owned()),
            eq("ENG".to_owned()),
        )
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _, _| Ok(Uuid::from_u128(6)));
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let response = post(
        router(work, MockRateLimiter::new()),
        &format!("/api/v1/workspaces/{SPACE}/teams"),
        json!({"name":" Engineering ","key":"eng"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn invitation_acceptance_keeps_oidc_login_method_and_rotates_cookie_after_commit() {
    let mut sequence = Sequence::new();
    let mut work = MockUnitOfWork::default();
    let mut rate_limiter = MockRateLimiter::new();
    rate_limiter.expect_check().times(1).returning(|_| Ok(()));
    work.invitations
        .expect_valid_token()
        .with(eq("invitation-token".to_owned()))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| {
            Ok(ValidInvite {
                id: Uuid::from_u128(7),
                workspace_id: SPACE,
                role: "member".into(),
                workspace_name: "Acme".into(),
                workspace_slug: "acme".into(),
            })
        });
    work.accounts
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| {
            Ok(User {
                id: ACTOR,
                display_name: "Existing account".into(),
                is_instance_admin: false,
            })
        });
    let identity = Uuid::from_u128(8);
    work.sessions
        .expect_session_method()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move |_| Ok(LoginMethod::Oidc(identity)));
    work.invitations
        .expect_accept()
        .withf(|invitation, user| invitation.workspace_id == SPACE && *user == ACTOR)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok(()));
    work.sessions
        .expect_revoke_all()
        .with(eq(ACTOR))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.sessions
        .expect_new_session()
        .withf(move |user, method| {
            *user == ACTOR && matches!(method, LoginMethod::Oidc(value) if *value == identity)
        })
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok(TOKEN.into()));
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let response = post(
        router(work, rate_limiter),
        "/api/v1/invitations/accept",
        json!({"token":"invitation-token"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .contains("HttpOnly; SameSite=Lax; Max-Age=604800")
    );
    assert_eq!(
        body(response).await,
        json!({"id":SPACE,"name":"Acme","slug":"acme","role":"member"})
    );
}

#[tokio::test]
async fn invitation_commit_failure_does_not_issue_session_cookie() {
    let mut work = MockUnitOfWork::default();
    let mut rate_limiter = MockRateLimiter::new();
    rate_limiter.expect_check().returning(|_| Ok(()));
    work.invitations.expect_valid_token().returning(|_| {
        Ok(ValidInvite {
            id: Uuid::from_u128(7),
            workspace_id: SPACE,
            role: "member".into(),
            workspace_name: "Acme".into(),
            workspace_slug: "acme".into(),
        })
    });
    work.accounts.expect_authenticate().returning(|_| {
        Ok(User {
            id: ACTOR,
            display_name: "Account".into(),
            is_instance_admin: false,
        })
    });
    work.sessions
        .expect_session_method()
        .returning(|_| Ok(LoginMethod::Local));
    work.invitations.expect_accept().returning(|_, _| Ok(()));
    work.sessions.expect_revoke_all().returning(|_| Ok(()));
    work.sessions
        .expect_new_session()
        .returning(|_, _| Ok(TOKEN.into()));
    work.lifecycle
        .expect_commit()
        .times(1)
        .returning(|| Err(anyhow::anyhow!("Commit failed")));
    let response = post(
        router(work, rate_limiter),
        "/api/v1/invitations/accept",
        json!({"token":"invitation-token"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!response.headers().contains_key("set-cookie"));
}

#[tokio::test]
async fn account_response_maps_membership_models_without_a_database() {
    let mut work = MockUnitOfWork::default();
    work.accounts.expect_authenticate().times(1).returning(|_| {
        Ok(User {
            id: ACTOR,
            display_name: "Account".into(),
            is_instance_admin: false,
        })
    });
    work.workspaces
        .expect_list_for_user()
        .with(eq(ACTOR))
        .times(1)
        .returning(|_| {
            Ok(vec![convex_workspaces::Workspace {
                id: SPACE,
                name: "Acme".into(),
                slug: "acme".into(),
                role: "member".into(),
            }])
        });
    work.lifecycle.expect_commit().times(1).returning(|| Ok(()));
    let response = router(work, MockRateLimiter::new())
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/me")
                .header("cookie", format!("convex_session={TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let result = body(response).await;
    assert_eq!(result["user"]["id"], ACTOR.to_string());
    assert_eq!(result["workspaces"][0]["role"], "member");
}

#[tokio::test]
async fn account_disablement_checks_workspace_owners_before_revoking_sessions() {
    let mut sequence = Sequence::new();
    let mut work = MockUnitOfWork::default();
    work.accounts
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| {
            Ok(User {
                id: ACTOR,
                display_name: "Administrator".into(),
                is_instance_admin: true,
            })
        });
    work.workspaces
        .expect_lock_all()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(vec![SPACE]));
    work.accounts
        .expect_find_for_update()
        .with(eq(TARGET))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| {
            Ok(User {
                id: TARGET,
                display_name: "Target".into(),
                is_instance_admin: false,
            })
        });
    work.memberships
        .expect_protect_last_owner()
        .with(eq(SPACE), eq(TARGET))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok(()));
    work.accounts
        .expect_set_disabled()
        .with(eq(TARGET), eq(true))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok(()));
    work.sessions
        .expect_revoke_all()
        .with(eq(TARGET))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let response = post(
        router(work, MockRateLimiter::new()),
        "/api/v1/admin/accounts/disable",
        json!({"user_id":TARGET,"disabled":true}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}
