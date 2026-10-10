use anyhow::Result;
use async_trait::async_trait;
use convex_db::postgres::PostgresTransaction;
use uuid::Uuid;

use crate::repository::SessionRepository;

#[async_trait]
impl SessionRepository for PostgresTransaction {
    async fn new_session(
        &mut self,
        user: Uuid,
        method: crate::security::LoginMethod,
    ) -> convex_core::Result<String> {
        security::new_session(self.transaction(), user, method).await
    }
    async fn session_method(
        &mut self,
        credentials: crate::credentials::Credentials,
    ) -> convex_core::Result<crate::security::LoginMethod> {
        security::session_method(self.transaction(), &credentials).await
    }
    async fn revoke_all(&mut self, user: Uuid) -> convex_core::Result<()> {
        accounts::revoke_all(self.transaction(), user).await
    }

    async fn active_user(&mut self, token_digest: &str) -> Result<Option<Uuid>> {
        // Preserve the issue authorization point: no last-activity writes or
        // account/session locks, which would conflict with administration's lock order.
        Ok(sqlx::query_scalar(
            "SELECT u.id
             FROM convex.users u
             JOIN convex.sessions s ON s.user_id = u.id
             WHERE s.token_hash = $1
               AND s.revoked_at IS NULL
               AND s.expires_at > now()
               AND u.disabled_at IS NULL",
        )
        .bind(token_digest)
        .fetch_optional(self.connection())
        .await?)
    }
}

pub mod accounts;
pub mod methods;
pub mod oidc;
pub mod policy;
pub mod security;

use crate::{repository::AuthenticationPolicy, security::SecurityConfig};
use std::sync::Arc;

/// A replica's shared authentication dependencies; transaction state is request-local.
#[derive(Clone)]
pub struct Context {
    pub pool: sqlx::PgPool,
    pub security: SecurityConfig,
    pub dummy_hash: String,
    pub oidc: Arc<oidc::Runtime>,
    pub authentication_policy: Arc<dyn AuthenticationPolicy>,
    pub invitations: Arc<dyn InvitationGateway>,
}

impl Context {
    pub async fn check_policy(&self) -> convex_core::Result<()> {
        self.authentication_policy
            .check()
            .await
            .map_err(convex_core::Error::internal)
    }
}

/// OIDC grants access only through the application's invitation workflow.
/// The caller retains the transaction, so identity and membership changes commit together.
#[async_trait::async_trait]
pub trait InvitationGateway: Send + Sync {
    async fn valid_token(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        token: &str,
    ) -> convex_core::Result<InvitationGrant>;
    async fn valid_id(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> convex_core::Result<InvitationGrant>;
    async fn accept(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        invitation: &InvitationGrant,
        user: Uuid,
    ) -> convex_core::Result<()>;
}

pub struct InvitationGrant {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub role: String,
    pub workspace_name: String,
    pub workspace_slug: String,
}

#[async_trait]
impl crate::repository::AccountRepository for PostgresTransaction {
    async fn authenticate(
        &mut self,
        credentials: crate::credentials::Credentials,
    ) -> convex_core::Result<crate::accounts::models::User> {
        accounts::authenticate(self.transaction(), &credentials).await
    }
    async fn create_local(
        &mut self,
        email: String,
        hash: String,
        display_name: String,
        admin: bool,
    ) -> convex_core::Result<crate::accounts::models::User> {
        accounts::create_local(self.transaction(), email, hash, display_name, admin).await
    }
    async fn find_for_update(
        &mut self,
        user: Uuid,
    ) -> convex_core::Result<crate::accounts::models::User> {
        sqlx::query_as(
            "SELECT id,display_name,is_instance_admin FROM convex.users WHERE id=$1 FOR UPDATE",
        )
        .bind(user)
        .fetch_optional(self.connection())
        .await?
        .ok_or_else(convex_core::Error::not_found)
    }
    async fn protect_enabled_administrators(&mut self, user: Uuid) -> convex_core::Result<()> {
        let admins:  i64  =  sqlx::query_scalar("SELECT count(*) FROM convex.users WHERE is_instance_admin AND disabled_at IS NULL AND id<>$1")
        .bind(user)
        .fetch_one(self.connection())
        .await?;
        if admins == 0 {
            return Err(convex_core::Error::conflict(
                "Keep an active instance administrator.",
            ));
        }
        let recovery_admins: bool = sqlx::query_scalar(
            "SELECT NOT EXISTS(SELECT 1
             FROM convex.local_credentials
             WHERE user_id=$1)
             OR EXISTS(SELECT 1
             FROM convex.users u
             JOIN convex.local_credentials c ON c.user_id=u.id
             WHERE u.id<>$1
             AND u.is_instance_admin
             AND u.disabled_at IS NULL)",
        )
        .bind(user)
        .fetch_one(self.connection())
        .await?;
        if !recovery_admins {
            return Err(convex_core::Error::conflict(
                "Keep an enabled instance administrator with local credentials for recovery.",
            ));
        }
        Ok(())
    }
    async fn set_disabled(&mut self, user: Uuid, disabled: bool) -> convex_core::Result<()> {
        sqlx::query("UPDATE convex.users SET disabled_at=CASE WHEN $2 THEN now() ELSE NULL END,updated_at=now() WHERE id=$1")
        .bind(user)
        .bind(disabled)
        .execute(self.connection())
        .await?;
        Ok(())
    }
}
#[async_trait]
impl crate::repository::InstallationRepository for PostgresTransaction {
    async fn initialized(&mut self) -> convex_core::Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT initialized_at IS NOT NULL FROM convex.installation FOR UPDATE",
        )
        .fetch_one(self.connection())
        .await?)
    }
    async fn mark_initialized(&mut self) -> convex_core::Result<()> {
        sqlx::query("UPDATE convex.installation SET initialized_at=now()")
            .execute(self.connection())
            .await?;
        Ok(())
    }
}

pub struct PostgresRateLimiter {
    pool: sqlx::PgPool,
}
impl PostgresRateLimiter {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl crate::repository::RateLimiter for PostgresRateLimiter {
    async fn check(&self, subject: &str) -> convex_core::Result<()> {
        security::rate_limit(&self.pool, subject).await
    }
}

mod services;
pub use services::Authentication;
