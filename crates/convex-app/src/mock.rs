use anyhow::Result;
use async_trait::async_trait;
use convex_auth::mock::MockAccountRepository;
use convex_auth::mock::MockInstallationRepository;
use convex_auth::repository::AccountRepository;
use convex_auth::repository::InstallationRepository;
use convex_auth::{mock::MockSessionRepository, repository::SessionRepository};
use convex_issues::{mock::MockIssueRepository, repository::IssueRepository};
use convex_workspaces::mock::MockInvitationRepository;
use convex_workspaces::mock::MockMembershipRepository;
use convex_workspaces::mock::MockTeamRepository;
use convex_workspaces::mock::MockWorkflowRepository;
use convex_workspaces::repository::InvitationRepository;
use convex_workspaces::repository::MembershipRepository;
use convex_workspaces::repository::TeamRepository;
use convex_workspaces::repository::WorkflowRepository;
use convex_workspaces::{mock::MockWorkspaceRepository, repository::WorkspaceRepository};

use crate::UnitOfWork;

pub use crate::unit_of_work::MockUnitOfWorkFactory;

/// Lifecycle expectations are separate from repository expectations, so endpoint
/// tests can assert that a mutation commits only after activity succeeds.
#[mockall::automock]
#[async_trait]
pub trait TransactionLifecycle: Send {
    async fn commit(&mut self) -> Result<()>;
    async fn rollback(&mut self) -> Result<()>;
}

#[derive(Default)]
pub struct MockUnitOfWork {
    pub accounts: MockAccountRepository,
    pub installation: MockInstallationRepository,
    pub memberships: MockMembershipRepository,
    pub teams: MockTeamRepository,
    pub workflows: MockWorkflowRepository,
    pub invitations: MockInvitationRepository,

    pub sessions: MockSessionRepository,
    pub workspaces: MockWorkspaceRepository,
    pub issues: MockIssueRepository,
    pub lifecycle: MockTransactionLifecycle,
}

#[async_trait]
impl UnitOfWork for MockUnitOfWork {
    fn accounts(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }
    fn installation(&mut self) -> &mut dyn InstallationRepository {
        &mut self.installation
    }
    fn memberships(&mut self) -> &mut dyn MembershipRepository {
        &mut self.memberships
    }
    fn teams(&mut self) -> &mut dyn TeamRepository {
        &mut self.teams
    }
    fn workflows(&mut self) -> &mut dyn WorkflowRepository {
        &mut self.workflows
    }
    fn invitations(&mut self) -> &mut dyn InvitationRepository {
        &mut self.invitations
    }

    fn sessions(&mut self) -> &mut dyn SessionRepository {
        &mut self.sessions
    }

    fn workspaces(&mut self) -> &mut dyn WorkspaceRepository {
        &mut self.workspaces
    }

    fn issues(&mut self) -> &mut dyn IssueRepository {
        &mut self.issues
    }

    async fn commit(mut self: Box<Self>) -> Result<()> {
        self.lifecycle.commit().await
    }

    async fn rollback(mut self: Box<Self>) -> Result<()> {
        self.lifecycle.rollback().await
    }
}
