use convex_ids::{InvitationId, TeamId, WorkflowStatusId};
use std::sync::{Arc, Mutex};

use chrono::{TimeZone, Utc};
use convex_auth::{
    mock::MockRateLimiter,
    security::{self, LoginMethod},
};
use convex_core::Error;
use convex_workspaces::*;
use mockall::{Sequence, predicate::eq};
use uuid::Uuid;

use super::{accept, change_member, create, detail, invite, preview, rename};
use crate::{accounts::models::AcceptInvite, mock::MockUnitOfWork, test_support::*};

fn invitation() -> ValidInvite {
    ValidInvite {
        id: InvitationId::from_uuid(Uuid::from_u128(7)),
        workspace_id: WORKSPACE,
        role: "member".into(),
        workspace_name: "Production".into(),
        workspace_slug: "production".into(),
    }
}

fn accepting_application(work: Option<MockUnitOfWork>) -> crate::control_plane::ControlPlane {
    let mut application = control_plane(work);
    let mut limiter = MockRateLimiter::new();
    limiter
        .expect_check()
        .with(eq(format!(
            "invitation-accept:{}",
            security::digest("invitation-token")
        )))
        .times(1)
        .returning(|_| Ok(()));
    application.rate_limiter = Arc::new(limiter);
    application
}

fn accept_request() -> AcceptInvite {
    AcceptInvite {
        token: "invitation-token".into(),
        email: None,
        password: None,
        display_name: None,
    }
}

#[tokio::test]
async fn workspace_creation_requires_instance_administration() {
    let mut work = MockUnitOfWork::default();
    expect_actor(&mut work, false);
    expect_rollback(&mut work);
    let request = CreateWorkspace {
        name: "Production".into(),
        slug: "production".into(),
        team_name: "Engineering".into(),
        team_key: "ENG".into(),
    };
    assert!(matches!(
        create(control_plane(Some(work)), credentials(), request).await,
        Err(Error::Forbidden)
    ));
}

#[tokio::test]
async fn workspace_detail_collects_each_teams_own_workflow() {
    let mut work = MockUnitOfWork::default();
    expect_actor(&mut work, false);
    expect_membership(&mut work, "member");
    work.workspaces
        .expect_identity()
        .with(eq(WORKSPACE))
        .times(1)
        .returning(|_| Ok(("Production".into(), "production".into())));
    work.memberships
        .expect_list()
        .with(eq(WORKSPACE))
        .times(1)
        .returning(|_| Ok(vec![]));
    let other_team = TeamId::from_uuid(Uuid::from_u128(8));
    work.teams
        .expect_list()
        .with(eq(WORKSPACE))
        .times(1)
        .returning(move |_| {
            Ok(vec![
                (TEAM, "Engineering".into(), "ENG".into()),
                (other_team, "Support".into(), "SUP".into()),
            ])
        });
    work.workflows
        .expect_for_team()
        .with(eq(TEAM))
        .times(1)
        .returning(|_| {
            Ok(vec![WorkflowStatus {
                id: WorkflowStatusId::from_uuid(Uuid::from_u128(9)),
                name: "Todo".into(),
                category: "unstarted".into(),
                position: 0,
            }])
        });
    work.workflows
        .expect_for_team()
        .with(eq(other_team))
        .times(1)
        .returning(|_| Ok(vec![]));
    expect_commit(&mut work);
    let result = detail(
        control_plane(Some(work)),
        credentials(),
        WORKSPACE.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(result.workspace.role, "member");
    assert_eq!(result.teams.len(), 2);
    assert_eq!(result.teams[0].statuses[0].name, "Todo");
    assert!(result.teams[1].statuses.is_empty());
}

#[tokio::test]
async fn ordinary_members_cannot_rename_workspaces() {
    let mut work = MockUnitOfWork::default();
    expect_actor(&mut work, false);
    expect_membership(&mut work, "member");
    expect_rollback(&mut work);
    let result = rename(
        control_plane(Some(work)),
        credentials(),
        WORKSPACE.to_string(),
        RenameRequest {
            name: "New name".into(),
        },
    )
    .await;
    assert!(matches!(result, Err(Error::Forbidden)));
}

#[tokio::test]
async fn membership_commands_require_exactly_one_action_before_starting_work() {
    for (remove, role) in [(false, None), (true, Some("member".into()))] {
        let result = change_member(
            control_plane(None),
            credentials(),
            WORKSPACE.to_string(),
            MembershipChange {
                membership_id: TARGET_MEMBER,
                remove,
                role,
            },
        )
        .await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }
}

#[tokio::test]
async fn workspace_admins_cannot_grant_administration_or_remove_owners() {
    for (target_role, remove, new_role) in [
        ("member", false, Some("admin".into())),
        ("owner", true, None),
    ] {
        let mut work = MockUnitOfWork::default();
        expect_actor(&mut work, false);
        expect_membership(&mut work, "admin");
        work.memberships
            .expect_target()
            .with(eq(WORKSPACE), eq(TARGET_MEMBER))
            .times(1)
            .returning(move |_, _| Ok((TARGET_USER, target_role.into())));
        expect_rollback(&mut work);
        let result = change_member(
            control_plane(Some(work)),
            credentials(),
            WORKSPACE.to_string(),
            MembershipChange {
                membership_id: TARGET_MEMBER,
                remove,
                role: new_role,
            },
        )
        .await;
        assert!(matches!(result, Err(Error::Forbidden)));
    }
}

#[tokio::test]
async fn granting_ownership_preserves_assignments_and_revokes_sessions_before_commit() {
    let mut work = MockUnitOfWork::default();
    let mut sequence = Sequence::new();
    expect_actor(&mut work, false);
    expect_membership(&mut work, "owner");
    work.memberships
        .expect_target()
        .with(eq(WORKSPACE), eq(TARGET_MEMBER))
        .times(1)
        .returning(|_, _| Ok((TARGET_USER, "member".into())));
    work.memberships
        .expect_change_role()
        .with(eq(TARGET_MEMBER), eq("owner".to_owned()))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok(()));
    work.sessions
        .expect_revoke_all()
        .with(eq(TARGET_USER))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let result = change_member(
        control_plane(Some(work)),
        credentials(),
        WORKSPACE.to_string(),
        MembershipChange {
            membership_id: TARGET_MEMBER,
            remove: false,
            role: Some("owner".into()),
        },
    )
    .await
    .unwrap();
    assert!(result.message.contains("sessions were revoked"));
}

#[tokio::test]
async fn workspace_admins_cannot_invite_other_admins() {
    let mut work = MockUnitOfWork::default();
    expect_actor(&mut work, false);
    expect_membership(&mut work, "admin");
    expect_rollback(&mut work);
    let result = invite(
        control_plane(Some(work)),
        credentials(),
        WORKSPACE.to_string(),
        InviteRequest {
            role: "admin".into(),
            deliver_to: None,
        },
    )
    .await;
    assert!(matches!(result, Err(Error::Forbidden)));
}

#[tokio::test]
async fn invitation_links_match_the_persisted_digest_without_storing_the_secret() {
    let mut work = MockUnitOfWork::default();
    let persisted = Arc::new(Mutex::new(None));
    let captured = persisted.clone();
    let expires = Utc.with_ymd_and_hms(2026, 10, 17, 12, 0, 0).unwrap();
    expect_actor(&mut work, false);
    expect_membership(&mut work, "owner");
    work.invitations
        .expect_create()
        .withf(|_, space, member, role, _| {
            *space == WORKSPACE && *member == ACTOR_MEMBER && role == "member"
        })
        .times(1)
        .returning(move |id, _, _, _, digest| {
            *captured.lock().unwrap() = Some((id, digest));
            Ok(expires)
        });
    expect_commit(&mut work);
    let result = invite(
        control_plane(Some(work)),
        credentials(),
        WORKSPACE.to_string(),
        InviteRequest {
            role: "member".into(),
            deliver_to: None,
        },
    )
    .await
    .unwrap();
    let secret = result
        .url
        .strip_prefix("http://127.0.0.1:5173/invite#")
        .unwrap();
    let (id, digest) = persisted.lock().unwrap().clone().unwrap();
    assert_eq!(result.id, id);
    assert_eq!(result.expires_at, expires);
    assert_eq!(security::digest(secret), digest);
    assert_ne!(secret, digest);
    assert!(!result.email_delivered);
}

#[tokio::test]
async fn email_invitation_requires_mail_configuration_before_starting_work() {
    let result = invite(
        control_plane(None),
        credentials(),
        WORKSPACE.to_string(),
        InviteRequest {
            role: "member".into(),
            deliver_to: Some("alex@example.com".into()),
        },
    )
    .await;
    assert!(matches!(result, Err(Error::Validation(_))));
}

#[tokio::test]
async fn preview_throttles_by_token_digest_and_returns_only_workspace_name_and_role() {
    let mut work = MockUnitOfWork::default();
    work.invitations
        .expect_valid_token()
        .with(eq("invitation-token".to_owned()))
        .times(1)
        .returning(|_| Ok(invitation()));
    expect_commit(&mut work);
    let mut application = control_plane(Some(work));
    let mut limiter = MockRateLimiter::new();
    limiter
        .expect_check()
        .with(eq(format!(
            "invitation-preview:{}",
            security::digest("invitation-token")
        )))
        .times(1)
        .returning(|_| Ok(()));
    application.rate_limiter = Arc::new(limiter);
    let result = preview(
        application,
        InviteToken {
            token: "invitation-token".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(result.workspace_name, "Production");
    assert_eq!(result.role, "member");
}

#[tokio::test]
async fn partial_signup_credentials_do_not_start_invitation_acceptance() {
    let mut request = accept_request();
    request.email = Some("alex@example.com".into());
    let result = accept(accepting_application(None), credentials(), request).await;
    assert!(matches!(result, Err(Error::Validation(_))));
}

#[tokio::test]
async fn new_invitees_get_local_accounts_and_sessions_in_the_acceptance_transaction() {
    let mut work = MockUnitOfWork::default();
    let mut sequence = Sequence::new();
    let password_hash = Arc::new(Mutex::new(String::new()));
    let captured_hash = password_hash.clone();
    work.invitations
        .expect_valid_token()
        .with(eq("invitation-token".to_owned()))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(invitation()));
    work.accounts
        .expect_create_local()
        .withf(|email, _, name, admin| email == "alex@example.com" && name == "Alex" && !admin)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move |_, hash, _, _| {
            *captured_hash.lock().unwrap() = hash;
            Ok(user(USER, false))
        });
    work.invitations
        .expect_accept()
        .withf(|invite, id| {
            invite.id == invitation().id && invite.workspace_id == WORKSPACE && *id == USER
        })
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok(()));
    work.sessions
        .expect_revoke_all()
        .with(eq(USER))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(()));
    work.sessions
        .expect_new_session()
        .withf(|id, method| *id == USER && matches!(method, LoginMethod::Local))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok("new-session".into()));
    work.lifecycle
        .expect_commit()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let mut request = accept_request();
    request.email = Some(" Alex@Example.com ".into());
    request.display_name = Some(" Alex ".into());
    request.password = Some("correct horse battery staple".into());
    let result = accept(accepting_application(Some(work)), credentials(), request)
        .await
        .unwrap();
    assert_eq!(result.token, "new-session");
    assert_eq!(result.value.id, WORKSPACE);
    assert_eq!(result.value.role, "member");
    let hash = password_hash.lock().unwrap().clone();
    security::verify_password("correct horse battery staple".into(), hash)
        .await
        .unwrap();
}

#[tokio::test]
async fn rejected_acceptance_preserves_existing_sessions_and_rolls_back() {
    let mut work = MockUnitOfWork::default();
    work.invitations
        .expect_valid_token()
        .times(1)
        .returning(|_| Ok(invitation()));
    expect_actor(&mut work, false);
    work.sessions
        .expect_session_method()
        .times(1)
        .returning(|_| Ok(LoginMethod::Local));
    work.invitations
        .expect_accept()
        .times(1)
        .returning(|_, _| Err(Error::conflict("already a member")));
    expect_rollback(&mut work);
    let result = accept(
        accepting_application(Some(work)),
        credentials(),
        accept_request(),
    )
    .await;
    assert!(matches!(result, Err(Error::Conflict(message)) if message == "already a member"));
}

#[tokio::test]
async fn removal_clears_assignments_before_deleting_membership_and_revoking_sessions() {
    // Revocation failure must abort the preceding issue and membership mutations too.
    for revocation_fails in [false, true] {
        let mut work = MockUnitOfWork::default();
        let mut sequence = Sequence::new();
        expect_actor(&mut work, false);
        expect_membership(&mut work, "owner");
        work.memberships
            .expect_target()
            .with(eq(WORKSPACE), eq(TARGET_MEMBER))
            .times(1)
            .returning(|_, _| Ok((TARGET_USER, "member".into())));
        work.memberships
            .expect_protect_last_owner()
            .with(eq(WORKSPACE), eq(TARGET_USER))
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_, _| Ok(()));
        work.issues
            .expect_clear_assignments()
            .with(eq(WORKSPACE), eq(TARGET_MEMBER), eq(ACTOR_MEMBER))
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_, _, _| Ok(()));
        work.memberships
            .expect_remove()
            .with(eq(TARGET_MEMBER))
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_| Ok(()));
        work.sessions
            .expect_revoke_all()
            .with(eq(TARGET_USER))
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_| {
                if revocation_fails {
                    Err(Error::Infrastructure(anyhow::anyhow!("revocation failed")))
                } else {
                    Ok(())
                }
            });
        if revocation_fails {
            work.lifecycle
                .expect_rollback()
                .times(1)
                .in_sequence(&mut sequence)
                .returning(|| Ok(()));
        } else {
            work.lifecycle
                .expect_commit()
                .times(1)
                .in_sequence(&mut sequence)
                .returning(|| Ok(()));
        }
        let result = change_member(
            control_plane(Some(work)),
            credentials(),
            WORKSPACE.to_string(),
            MembershipChange {
                membership_id: TARGET_MEMBER,
                role: None,
                remove: true,
            },
        )
        .await;
        if revocation_fails {
            assert!(
                matches!(result, Err(Error::Infrastructure(error)) if error.to_string() == "revocation failed")
            );
        } else {
            assert!(result.is_ok());
        }
    }
}
