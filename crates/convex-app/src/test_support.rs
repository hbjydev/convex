use convex_ids::{MembershipId, TeamId, UserId, WorkspaceId};
use std::sync::Arc;

use convex_auth::{
    accounts::models::User, credentials::Credentials, mock::MockRateLimiter,
    security::SecurityConfig,
};
use mockall::predicate::eq;
use uuid::Uuid;

use crate::{
    control_plane::ControlPlane,
    mock::{MockUnitOfWork, MockUnitOfWorkFactory},
};

pub const USER: UserId = UserId::from_uuid(Uuid::from_u128(1));
pub const ACTOR_MEMBER: MembershipId = MembershipId::from_uuid(Uuid::from_u128(2));
pub const WORKSPACE: WorkspaceId = WorkspaceId::from_uuid(Uuid::from_u128(3));
pub const TARGET_USER: UserId = UserId::from_uuid(Uuid::from_u128(4));
pub const TARGET_MEMBER: MembershipId = MembershipId::from_uuid(Uuid::from_u128(5));
pub const TEAM: TeamId = TeamId::from_uuid(Uuid::from_u128(6));

pub fn user(id: UserId, admin: bool) -> User {
    User {
        id,
        display_name: "Alex".into(),
        is_instance_admin: admin,
    }
}

pub fn credentials() -> Credentials {
    Credentials {
        session: Some("session-token".into()),
        ..Default::default()
    }
}

/// No expectation means unexpected persistence or throttling panics immediately.
pub fn control_plane(work: Option<MockUnitOfWork>) -> ControlPlane {
    let mut transactions = MockUnitOfWorkFactory::new();
    if let Some(work) = work {
        transactions
            .expect_begin_administration()
            .times(1)
            .return_once(move || Ok(Box::new(work)));
    }
    ControlPlane {
        transactions: Arc::new(transactions),
        security: SecurityConfig::development(),
        rate_limiter: Arc::new(MockRateLimiter::new()),
    }
}

pub fn expect_actor(work: &mut MockUnitOfWork, admin: bool) {
    work.accounts
        .expect_authenticate()
        .withf(|credentials| credentials.session.as_deref() == Some("session-token"))
        .times(1)
        .returning(move |_| Ok(user(USER, admin)));
}

pub fn expect_membership(work: &mut MockUnitOfWork, role: &'static str) {
    work.memberships
        .expect_for_user()
        .with(eq(WORKSPACE), eq(USER))
        .times(1)
        .returning(move |_, _| Ok((ACTOR_MEMBER, role.into())));
}

pub fn expect_commit(work: &mut MockUnitOfWork) {
    work.lifecycle.expect_commit().times(1).returning(|| Ok(()));
}

pub fn expect_rollback(work: &mut MockUnitOfWork) {
    work.lifecycle
        .expect_rollback()
        .times(1)
        .returning(|| Ok(()));
}
