use crate::auth_policy::*;
use anyhow::{Result, ensure};
use convex_core::{Error as AppError, Result as AppResult};
use convex_ids::UserId;
use sqlx::{Connection, PgConnection, PgPool, Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};
const LOCK_NAMESPACE: i32 = 0x435658;
const LOCK_POLICY: i32 = 3;
pub struct ActivePolicy {
    connection: Mutex<PgConnection>,
    fingerprint: String,
}

/// Activates once per replica and checks its dedicated policy lease on each request.
pub struct PolicyGuard {
    pool: PgPool,
    configuration: AuthenticationConfig,
    active: Arc<OnceCell<ActivePolicy>>,
}

impl PolicyGuard {
    pub fn new(
        pool: PgPool,
        configuration: AuthenticationConfig,
        active: Arc<OnceCell<ActivePolicy>>,
    ) -> Self {
        Self {
            pool,
            configuration,
            active,
        }
    }
}

#[async_trait::async_trait]
impl crate::repository::AuthenticationPolicy for PolicyGuard {
    async fn check(&self) -> Result<()> {
        let policy = self
            .active
            .get_or_try_init(|| activate(&self.pool, &self.configuration))
            .await?;
        policy.check().await
    }
}

impl ActivePolicy {
    /// A failed lease connection makes this replica unavailable for authentication.
    /// It cannot silently reconnect and continue with stale configuration.
    pub async fn check(&self) -> Result<()> {
        let fingerprint: Option<String> = sqlx::query_scalar(
            "SELECT fingerprint FROM convex.authentication_policy WHERE singleton",
        )
        .fetch_one(&mut *self.connection.lock().await)
        .await?;
        ensure!(
            fingerprint.as_deref() == Some(&self.fingerprint),
            "Authentication configuration changed; restart this replica"
        );
        Ok(())
    }
}

pub async fn activate(pool: &PgPool, config: &AuthenticationConfig) -> Result<ActivePolicy> {
    let fingerprint = config.fingerprint();
    let provider_revision = config.provider_revision();
    let exchange_revision = config.exchange_revision();
    let mut connection = pool.acquire().await?.detach();
    let mut tx = connection.begin().await?;
    sqlx::query("SELECT singleton FROM convex.installation FOR UPDATE")
        .execute(&mut *tx)
        .await?;
    let (old_fingerprint, old_ttl): (Option<String>, i64) = sqlx::query_as(
        "SELECT fingerprint, session_ttl_seconds FROM convex.authentication_policy WHERE singleton FOR UPDATE"
    )
        .fetch_one(&mut *tx)
        .await?;
    let changed = old_fingerprint.as_deref() != Some(&fingerprint);
    if changed {
        let exclusive: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1,$2)")
            .bind(LOCK_NAMESPACE)
            .bind(LOCK_POLICY)
            .fetch_one(&mut *tx)
            .await?;
        ensure!(
            exclusive,
            "Authentication settings differ from running replicas. Stop all replicas before changing authentication environment variables."
        );
        if i64::from(config.session_lifetime.seconds()) < old_ttl {
            sqlx::query("UPDATE convex.sessions
             SET expires_at=LEAST(expires_at, created_at + make_interval(secs => $1::bigint::double precision))
             WHERE revoked_at IS NULL")
                .bind(i64::from(config.session_lifetime.seconds()))
        .execute(&mut *tx)
        .await?;
        }
        sqlx::query(
            "UPDATE convex.sessions s
             SET revoked_at=now()
             FROM convex.users u
             WHERE s.user_id=u.id
             AND s.revoked_at IS NULL
             AND ((s.login_method= 'local'
             AND $1
             AND NOT u.is_instance_admin)
             OR (s.login_method= 'oidc'
             AND s.provider_revision IS DISTINCT
             FROM $2))",
        )
        .bind(config.oidc_required)
        .bind(&provider_revision)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE
             FROM convex.oidc_flows
             WHERE exchange_revision IS DISTINCT
             FROM $1
             OR expires_at<=now()
             OR session_id IN (SELECT id
             FROM convex.sessions
             WHERE revoked_at IS NOT NULL
             OR expires_at<=now())",
        )
        .bind(&exchange_revision)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE
             FROM convex.method_proofs
             WHERE exchange_revision IS DISTINCT
             FROM $1
             OR expires_at<=now()
             OR session_id IN (SELECT id
             FROM convex.sessions
             WHERE revoked_at IS NOT NULL
             OR expires_at<=now())",
        )
        .bind(&exchange_revision)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE convex.authentication_policy
             SET
             fingerprint=$1,provider_revision=$2,exchange_revision=$3,oidc_issuer=$4,oidc_enabled=$5,oidc_required=$6,session_ttl_seconds=$7
             WHERE singleton")
            .bind(&fingerprint)
        .bind(&provider_revision)
        .bind(&exchange_revision)
            .bind(config.provider.as_ref().map(|p| &p.issuer))
            .bind(config.provider.as_ref().is_some_and(|p| p.enabled))
            .bind(config.oidc_required)
        .bind(i64::from(config.session_lifetime.seconds()))
            .execute(&mut *tx)
        .await?;
    }
    let shared: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock_shared($1,$2)")
        .bind(LOCK_NAMESPACE)
        .bind(LOCK_POLICY)
        .fetch_one(&mut *tx)
        .await?;
    ensure!(
        shared,
        "Another replica is updating authentication settings"
    );
    if changed {
        sqlx::query("SELECT pg_advisory_unlock($1,$2)")
            .bind(LOCK_NAMESPACE)
            .bind(LOCK_POLICY)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(ActivePolicy {
        connection: Mutex::new(connection),
        fingerprint,
    })
}

pub async fn ensure_local_allowed(
    tx: &mut Transaction<'_, Postgres>,
    user: UserId,
) -> AppResult<()> {
    let allowed: bool = sqlx::query_scalar(
        "SELECT NOT p.oidc_required
             OR u.is_instance_admin
             FROM convex.users u
             CROSS JOIN convex.authentication_policy p
             WHERE u.id=$1",
    )
    .bind(user)
    .fetch_one(&mut **tx)
    .await?;
    if allowed {
        Ok(())
    } else {
        Err(AppError::forbidden())
    }
}
