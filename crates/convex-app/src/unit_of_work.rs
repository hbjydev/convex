use anyhow::Result;
use async_trait::async_trait;
use convex_auth::repository::AccountRepository;
use convex_auth::repository::InstallationRepository;
use convex_auth::repository::SessionRepository;
use convex_issues::repository::IssueRepository;
use convex_workspaces::repository::InvitationRepository;
use convex_workspaces::repository::MembershipRepository;
use convex_workspaces::repository::TeamRepository;
use convex_workspaces::repository::WorkflowRepository;
use convex_workspaces::repository::WorkspaceRepository;

/// Starts an uncommitted transaction. Application operations acquire the locks
/// appropriate to their workflow; issue operations never take the installation gate.
#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin_administration(&self) -> Result<Box<dyn UnitOfWork>>;
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>>;
}

/// All repositories operate on the same transaction. Mutable access exposes one
/// repository at a time and prevents concurrent use of its connection.
/// Commit/rollback consume the carrier; dropping it must abort unfinished work.
#[async_trait]
pub trait UnitOfWork: Send {
    fn accounts(&mut self) -> &mut dyn AccountRepository;
    fn installation(&mut self) -> &mut dyn InstallationRepository;
    fn memberships(&mut self) -> &mut dyn MembershipRepository;
    fn teams(&mut self) -> &mut dyn TeamRepository;
    fn workflows(&mut self) -> &mut dyn WorkflowRepository;
    fn invitations(&mut self) -> &mut dyn InvitationRepository;
    fn sessions(&mut self) -> &mut dyn SessionRepository;
    fn workspaces(&mut self) -> &mut dyn WorkspaceRepository;
    fn issues(&mut self) -> &mut dyn IssueRepository;
    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}
