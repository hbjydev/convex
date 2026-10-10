use crate::{
    IssueApplication,
    services::{Readiness, Services},
};
use async_trait::async_trait;
use convex_auth::{
    postgres::{Context, InvitationGateway, InvitationGrant},
    security::SecurityConfig,
};
use convex_core::Result;
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::OnceCell;
use uuid::Uuid;

#[derive(Clone)]
pub struct ApplicationBackend {
    control_plane: crate::control_plane::ControlPlane,
    context: Context,
    active_policy: Arc<OnceCell<convex_auth::postgres::policy::ActivePolicy>>,
}

impl ApplicationBackend {
    pub fn new(pool: PgPool, security: SecurityConfig) -> Self {
        let active_policy = Arc::new(OnceCell::new());
        let authentication_policy = Arc::new(convex_auth::postgres::policy::PolicyGuard::new(
            pool.clone(),
            security.authentication.clone(),
            active_policy.clone(),
        ));
        let control_plane = crate::control_plane::ControlPlane {
            transactions: Arc::new(super::PostgresUnitOfWorkFactory::new(pool.clone())),
            security: security.clone(),
            rate_limiter: Arc::new(convex_auth::postgres::PostgresRateLimiter::new(
                pool.clone(),
            )),
        };
        Self {
            control_plane,
            context: Context {
                pool,
                security,
                dummy_hash: convex_auth::security::dummy_password_hash(),
                oidc: Arc::new(convex_auth::postgres::oidc::Runtime::default()),
                authentication_policy,
                invitations: Arc::new(Invitations),
            },
            active_policy,
        }
    }

    pub async fn prepare(&self) -> anyhow::Result<()> {
        self.context.authentication_policy.check().await
    }

    pub fn services(self) -> Services {
        let issues = IssueApplication::new(Arc::new(super::PostgresUnitOfWorkFactory::new(
            self.context.pool.clone(),
        )));
        let authentication_policy = self.context.authentication_policy.clone();
        let authentication = Arc::new(convex_auth::postgres::Authentication {
            context: self.context.clone(),
        });
        let accounts = Arc::new(crate::accounts::AccountApplication {
            control_plane: self.control_plane.clone(),
            authentication: authentication.clone(),
        });
        let workspaces = Arc::new(self.control_plane.clone());
        Services {
            accounts,
            workspaces,
            login_methods: authentication.clone(),
            oidc: authentication,
            issues,
            authentication_policy,
            readiness: Arc::new(self),
        }
    }
}

#[async_trait]
impl Readiness for ApplicationBackend {
    async fn check(&self) -> anyhow::Result<()> {
        if let Some(policy) = self.active_policy.get() {
            policy.check().await?;
        }
        convex_db::postgres::schema_ready(&self.context.pool).await
    }
}

struct Invitations;
#[async_trait]
impl InvitationGateway for Invitations {
    async fn valid_token(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        token: &str,
    ) -> Result<InvitationGrant> {
        Ok(invitation_grant(
            convex_workspaces::postgres::administration::valid_invite(tx, token).await?,
        ))
    }
    async fn valid_id(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
    ) -> Result<InvitationGrant> {
        Ok(invitation_grant(
            convex_workspaces::postgres::administration::valid_invite_id(tx, id).await?,
        ))
    }
    async fn accept(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        invitation: &InvitationGrant,
        user: Uuid,
    ) -> Result<()> {
        let invitation = convex_workspaces::ValidInvite {
            id: invitation.id,
            workspace_id: invitation.workspace_id,
            role: invitation.role.clone(),
            workspace_name: invitation.workspace_name.clone(),
            workspace_slug: invitation.workspace_slug.clone(),
        };
        convex_workspaces::postgres::administration::accept_for_user(tx, &invitation, user).await
    }
}

fn invitation_grant(invitation: convex_workspaces::ValidInvite) -> InvitationGrant {
    InvitationGrant {
        id: invitation.id,
        workspace_id: invitation.workspace_id,
        role: invitation.role,
        workspace_name: invitation.workspace_name,
        workspace_slug: invitation.workspace_slug,
    }
}
