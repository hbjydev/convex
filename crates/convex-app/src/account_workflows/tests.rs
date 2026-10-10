use std::sync::{Arc, Mutex};

use convex_auth::{
    mock::MockRateLimiter,
    security::{self, LoginMethod},
};
use convex_core::Error;
use mockall::{Sequence, predicate::eq};

use super::{disable, me, setup};
use crate::{
    accounts::models::{DisableRequest, SetupRequest},
    control_plane::ControlPlane,
    mock::MockUnitOfWork,
    test_support::*,
};

const PASSWORD: &str = "correct horse battery staple";

fn setup_request() -> SetupRequest {
    SetupRequest {
        setup_token: "setup-secret".into(),
        email: " Alex@Example.com ".into(),
        password: PASSWORD.into(),
        display_name: " Alex ".into(),
        workspace_name: " Production ".into(),
        workspace_slug: "production".into(),
        team_name: " Engineering ".into(),
        team_key: "eng".into(),
    }
}

fn setup_application(work: Option<MockUnitOfWork>) -> ControlPlane {
    let mut application = control_plane(work);
    application.security.setup_token = Some("setup-secret".into());
    let mut limiter = MockRateLimiter::new();
    limiter
        .expect_check()
        .with(eq("setup"))
        .times(1)
        .returning(|_| Ok(()));
    application.rate_limiter = Arc::new(limiter);
    application
}

#[tokio::test]
async fn setup_requires_the_configured_secret_before_starting_work() {
    for configured in [None, Some("different-secret".into())] {
        let mut application = setup_application(None);
        application.security.setup_token = configured;
        assert!(matches!(
            setup(application, setup_request()).await,
            Err(Error::Forbidden)
        ));
    }
}

#[tokio::test]
async fn throttled_setup_does_not_start_work() {
    let mut application = control_plane(None);
    let mut limiter = MockRateLimiter::new();
    limiter
        .expect_check()
        .with(eq("setup"))
        .times(1)
        .returning(|_| Err(Error::RateLimited));
    application.rate_limiter = Arc::new(limiter);
    assert!(matches!(
        setup(application, setup_request()).await,
        Err(Error::RateLimited)
    ));
}

#[tokio::test]
async fn setup_creates_the_admin_workspace_and_session_in_one_transaction() {
    let mut work = MockUnitOfWork::default();
    let mut sequence = Sequence::new();
    let password_hash = Arc::new(Mutex::new(String::new()));
    let captured_hash = password_hash.clone();
    work.installation
        .expect_initialized()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(false));
    work.accounts
        .expect_create_local()
        .withf(|email, _, name, admin| email == "alex@example.com" && name == "Alex" && *admin)
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move |_, hash, _, _| {
            *captured_hash.lock().unwrap() = hash;
            Ok(user(USER, true))
        });
    work.workspaces
        .expect_create()
        .with(
            eq(USER),
            eq("Production".to_owned()),
            eq("production".to_owned()),
            eq("Engineering".to_owned()),
            eq("ENG".to_owned()),
        )
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _, _, _, _| Ok(WORKSPACE));
    work.installation
        .expect_mark_initialized()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
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

    let result = setup(setup_application(Some(work)), setup_request())
        .await
        .unwrap();

    assert_eq!(result.value.id, USER);
    assert!(result.value.is_instance_admin);
    assert_eq!(result.token, "new-session");
    assert!(!result.clear_method_proof);
    let hash = password_hash.lock().unwrap().clone();
    assert_ne!(hash, PASSWORD);
    security::verify_password(PASSWORD.into(), hash)
        .await
        .unwrap();
}

#[tokio::test]
async fn initialized_installations_reject_setup_without_creating_accounts() {
    let mut work = MockUnitOfWork::default();
    work.installation
        .expect_initialized()
        .times(1)
        .returning(|| Ok(true));
    expect_rollback(&mut work);
    assert!(matches!(
        setup(setup_application(Some(work)), setup_request()).await,
        Err(Error::Conflict(_))
    ));
}

#[tokio::test]
async fn failed_initial_workspace_rolls_back_the_account_without_initializing_or_signing_in() {
    let mut work = MockUnitOfWork::default();
    work.installation
        .expect_initialized()
        .times(1)
        .returning(|| Ok(false));
    work.accounts
        .expect_create_local()
        .times(1)
        .returning(|_, _, _, _| Ok(user(USER, true)));
    work.workspaces
        .expect_create()
        .times(1)
        .returning(|_, _, _, _, _| Err(Error::conflict("workspace slug taken")));
    expect_rollback(&mut work);
    let result = setup(setup_application(Some(work)), setup_request()).await;
    assert!(matches!(result, Err(Error::Conflict(message)) if message == "workspace slug taken"));
}

#[tokio::test]
async fn me_rolls_back_when_workspace_lookup_fails() {
    let mut work = MockUnitOfWork::default();
    expect_actor(&mut work, false);
    work.workspaces
        .expect_list_for_user()
        .with(eq(USER))
        .times(1)
        .returning(|_| {
            Err(Error::Infrastructure(anyhow::anyhow!(
                "workspace lookup failed"
            )))
        });
    expect_rollback(&mut work);
    assert!(matches!(
        me(control_plane(Some(work)), credentials()).await,
        Err(Error::Infrastructure(_))
    ));
}

#[tokio::test]
async fn only_instance_admins_can_disable_accounts() {
    let mut work = MockUnitOfWork::default();
    expect_actor(&mut work, false);
    expect_rollback(&mut work);
    let result = disable(
        control_plane(Some(work)),
        credentials(),
        DisableRequest {
            user_id: TARGET_USER,
            disabled: true,
        },
    )
    .await;
    assert!(matches!(result, Err(Error::Forbidden)));
}

#[tokio::test]
async fn instance_admins_cannot_disable_their_own_account() {
    let mut work = MockUnitOfWork::default();
    expect_actor(&mut work, true);
    expect_rollback(&mut work);
    let result = disable(
        control_plane(Some(work)),
        credentials(),
        DisableRequest {
            user_id: USER,
            disabled: true,
        },
    )
    .await;
    assert!(matches!(result, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn disabling_the_final_instance_admin_rolls_back_before_changing_the_account() {
    let mut work = MockUnitOfWork::default();
    expect_actor(&mut work, true);
    work.workspaces
        .expect_lock_all()
        .times(1)
        .returning(|| Ok(vec![WORKSPACE]));
    work.accounts
        .expect_find_for_update()
        .with(eq(TARGET_USER))
        .times(1)
        .returning(|_| Ok(user(TARGET_USER, true)));
    work.accounts
        .expect_protect_enabled_administrators()
        .with(eq(TARGET_USER))
        .times(1)
        .returning(|_| Err(Error::conflict("last enabled administrator")));
    expect_rollback(&mut work);
    let result = disable(
        control_plane(Some(work)),
        credentials(),
        DisableRequest {
            user_id: TARGET_USER,
            disabled: true,
        },
    )
    .await;
    assert!(
        matches!(result, Err(Error::Conflict(message)) if message == "last enabled administrator")
    );
}

#[tokio::test]
async fn enabling_an_account_skips_disablement_guards_and_revokes_old_sessions() {
    let mut work = MockUnitOfWork::default();
    let mut sequence = Sequence::new();
    expect_actor(&mut work, true);
    work.workspaces
        .expect_lock_all()
        .times(1)
        .returning(|| Ok(vec![WORKSPACE]));
    work.accounts
        .expect_find_for_update()
        .with(eq(TARGET_USER))
        .times(1)
        .returning(|_| Ok(user(TARGET_USER, true)));
    work.accounts
        .expect_set_disabled()
        .with(eq(TARGET_USER), eq(false))
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
    let result = disable(
        control_plane(Some(work)),
        credentials(),
        DisableRequest {
            user_id: TARGET_USER,
            disabled: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        result.message,
        "Account enabled. Sign in again to continue."
    );
}

#[tokio::test]
async fn disabling_checks_every_workspace_owner_before_changing_the_account() {
    let mut work = MockUnitOfWork::default();
    let mut sequence = Sequence::new();
    let other_workspace = uuid::Uuid::from_u128(10);
    expect_actor(&mut work, true);
    work.workspaces
        .expect_lock_all()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(move || Ok(vec![WORKSPACE, other_workspace]));
    work.accounts
        .expect_find_for_update()
        .with(eq(TARGET_USER))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Ok(user(TARGET_USER, false)));
    work.memberships
        .expect_protect_last_owner()
        .with(eq(WORKSPACE), eq(TARGET_USER))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Ok(()));
    work.memberships
        .expect_protect_last_owner()
        .with(eq(other_workspace), eq(TARGET_USER))
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_, _| Err(Error::conflict("last workspace owner")));
    work.lifecycle
        .expect_rollback()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| Ok(()));
    let result = disable(
        control_plane(Some(work)),
        credentials(),
        DisableRequest {
            user_id: TARGET_USER,
            disabled: true,
        },
    )
    .await;
    assert!(matches!(result, Err(Error::Conflict(message)) if message == "last workspace owner"));
}
