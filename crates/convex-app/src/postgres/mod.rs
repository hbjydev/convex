use anyhow::Result;
use async_trait::async_trait;
use convex_auth::repository::AccountRepository;
use convex_auth::repository::InstallationRepository;
use convex_auth::repository::SessionRepository;
use convex_db::postgres::PostgresTransaction;
use convex_issues::repository::IssueRepository;
use convex_workspaces::repository::InvitationRepository;
use convex_workspaces::repository::MembershipRepository;
use convex_workspaces::repository::TeamRepository;
use convex_workspaces::repository::WorkflowRepository;
use convex_workspaces::repository::WorkspaceRepository;

use crate::{UnitOfWork, UnitOfWorkFactory};

pub struct PostgresUnitOfWorkFactory {
    pool: sqlx::PgPool,
}

impl PostgresUnitOfWorkFactory {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UnitOfWorkFactory for PostgresUnitOfWorkFactory {
    async fn begin_administration(&self) -> Result<Box<dyn UnitOfWork>> {
        Ok(Box::new(
            PostgresTransaction::begin_administration(&self.pool).await?,
        ))
    }

    async fn begin(&self) -> Result<Box<dyn UnitOfWork>> {
        Ok(Box::new(PostgresTransaction::begin(&self.pool).await?))
    }
}

// Each domain implements its own repository trait for this carrier. Every getter
// borrows the same transaction, with no connection pools hidden in repositories.
#[async_trait]
impl UnitOfWork for PostgresTransaction {
    fn accounts(&mut self) -> &mut dyn AccountRepository {
        self
    }
    fn installation(&mut self) -> &mut dyn InstallationRepository {
        self
    }
    fn memberships(&mut self) -> &mut dyn MembershipRepository {
        self
    }
    fn teams(&mut self) -> &mut dyn TeamRepository {
        self
    }
    fn workflows(&mut self) -> &mut dyn WorkflowRepository {
        self
    }
    fn invitations(&mut self) -> &mut dyn InvitationRepository {
        self
    }

    fn sessions(&mut self) -> &mut dyn SessionRepository {
        self
    }

    fn workspaces(&mut self) -> &mut dyn WorkspaceRepository {
        self
    }

    fn issues(&mut self) -> &mut dyn IssueRepository {
        self
    }

    async fn commit(self: Box<Self>) -> Result<()> {
        PostgresTransaction::commit(*self).await
    }

    async fn rollback(self: Box<Self>) -> Result<()> {
        PostgresTransaction::rollback(*self).await
    }
}

mod services;
pub use services::ApplicationBackend;
