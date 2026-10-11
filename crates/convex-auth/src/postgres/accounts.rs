use crate::{
    accounts::models::*,
    credentials::{Credentials, SessionUpdate},
    postgres::{Context, security as storage},
    security as sec,
};
use chrono::{DateTime, Utc};
use convex_core::{Error as AppError, Result as AppResult};
use convex_ids::{RecoveryTokenId, UserId};
use sqlx::{Postgres, Transaction};

fn with_session<T>(_state: &Context, token: &str, value: T) -> SessionUpdate<T> {
    SessionUpdate::new(token, value)
}

/// CP02 control-plane operations share a transaction gate. This gives role changes,
/// disablement, login, and token consumption a consistent lock order across replicas.
/// Issue operations in CP04 must use scoped locks rather than this administration gate.
pub async fn transaction(pool: &sqlx::PgPool) -> AppResult<Transaction<'_, Postgres>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT singleton FROM convex.installation FOR UPDATE")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

pub async fn authenticate(
    tx: &mut Transaction<'_, Postgres>,
    credentials: &Credentials,
) -> AppResult<User> {
    let hash = sec::digest(&sec::session_token(credentials)?);
    let user = sqlx::query_as::<_, User>(
        "SELECT u.id,u.display_name,u.is_instance_admin
             FROM convex.users u
             JOIN convex.sessions s ON s.user_id=u.id
             CROSS JOIN convex.authentication_policy p
             LEFT JOIN convex.oidc_identities i ON i.id=s.oidc_identity_id
             AND i.user_id=u.id
             WHERE s.token_hash=$1
             AND s.revoked_at IS NULL
             AND s.expires_at>now()
             AND u.disabled_at IS NULL
             AND ((s.login_method= 'local'
             AND (NOT p.oidc_required
             OR u.is_instance_admin)
             AND EXISTS(SELECT 1
             FROM convex.local_credentials c
             WHERE c.user_id=u.id))
             OR (s.login_method= 'oidc'
             AND p.oidc_enabled
             AND s.provider_revision=p.provider_revision
             AND i.unlinked_at IS NULL
             AND i.issuer=p.oidc_issuer))
             FOR UPDATE OF u,s",
    )
    .bind(&hash)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(AppError::unauthorized)?;
    sqlx::query("UPDATE convex.sessions SET last_activity_at=now() WHERE token_hash=$1")
        .bind(hash)
        .execute(&mut **tx)
        .await?;
    Ok(user)
}

pub async fn revoke_all(tx: &mut Transaction<'_, Postgres>, user: UserId) -> AppResult<()> {
    sqlx::query(
        "UPDATE convex.sessions SET revoked_at=now() WHERE user_id=$1 AND revoked_at IS NULL",
    )
    .bind(user)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "DELETE
             FROM convex.oidc_flows
             WHERE user_id=$1
             OR session_id IN (SELECT id
             FROM convex.sessions
             WHERE user_id=$1)",
    )
    .bind(user)
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM convex.method_proofs WHERE user_id=$1")
        .bind(user)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn create_local(
    tx: &mut Transaction<'_, Postgres>,
    email: String,
    hash: String,
    display_name: String,
    admin: bool,
) -> AppResult<User> {
    let user = User {
        id: UserId::new(),
        display_name,
        is_instance_admin: admin,
    };
    sqlx::query("INSERT INTO convex.users (id,display_name,is_instance_admin) VALUES ($1,$2,$3)")
        .bind(user.id)
        .bind(&user.display_name)
        .bind(admin)
        .execute(&mut **tx)
        .await?;
    crate::postgres::policy::ensure_local_allowed(tx, user.id).await?;
    sqlx::query(
        "INSERT INTO convex.local_credentials (user_id,email,password_hash) VALUES ($1,$2,$3)",
    )
    .bind(user.id)
    .bind(email)
    .bind(hash)
    .execute(&mut **tx)
    .await?;
    Ok(user)
}

pub async fn bootstrap(context: Context) -> AppResult<BootstrapInfo> {
    let initialized: bool =
        sqlx::query_scalar("SELECT initialized_at IS NOT NULL FROM convex.installation")
            .fetch_one(&context.pool)
            .await?;
    Ok(BootstrapInfo {
        setup_required: !initialized,
        setup_enabled: !initialized && context.security.setup_token.is_some(),
        mail_available: context.security.mail.is_some(),
        oidc_display_name: context
            .security
            .authentication
            .provider
            .as_ref()
            .filter(|p| p.enabled)
            .map(|p| p.display_name.clone()),
        oidc_required: context.security.authentication.oidc_required,
    })
}

pub async fn login(
    context: Context,
    credentials: Credentials,
    req: LoginRequest,
) -> AppResult<SessionUpdate<User>> {
    let email = sec::email(&req.email)?;
    storage::rate_limit(&context.pool, &format!("login:{email}")).await?;
    let mut tx = transaction(&context.pool).await?;
    let record = sqlx::query_as::<_, (UserId, String, Option<DateTime<Utc>>)>(
        "SELECT u.id,c.password_hash,u.disabled_at
             FROM convex.users u
             JOIN convex.local_credentials c ON c.user_id=u.id
             WHERE c.email=$1
             FOR UPDATE OF u,c",
    )
    .bind(email)
    .fetch_optional(&mut *tx)
    .await?;
    let dummy = context.dummy_hash.clone();
    sec::verify_password(
        req.password,
        record.as_ref().map(|r| r.1.clone()).unwrap_or(dummy),
    )
    .await?;
    let (id, _, disabled) = record.ok_or_else(AppError::unauthorized)?;
    if disabled.is_some() {
        return Err(AppError::unauthorized());
    }
    // Rotate any browser session, even when switching accounts.
    if let Ok(old) = sec::session_token(&credentials) {
        sqlx::query("DELETE FROM convex.method_proofs WHERE session_id IN (SELECT id FROM convex.sessions WHERE token_hash=$1)")
        .bind(sec::digest(&old))
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE convex.sessions SET revoked_at=now() WHERE token_hash=$1")
            .bind(sec::digest(&old))
            .execute(&mut *tx)
            .await?;
    }
    let token = storage::new_session(&mut tx, id, sec::LoginMethod::Local).await?;
    let user = sqlx::query_as::<_, User>(
        "SELECT id,display_name,is_instance_admin FROM convex.users WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(with_session(&context, &token, user))
}

pub async fn logout(
    context: Context,
    credentials: Credentials,
) -> AppResult<SessionUpdate<MessageResponse>> {
    if let Ok(token) = sec::session_token(&credentials) {
        let mut tx = transaction(&context.pool).await?;
        sqlx::query("DELETE FROM convex.oidc_flows WHERE session_id IN (SELECT id FROM convex.sessions WHERE token_hash=$1)")
            .bind(sec::digest(&token))
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM convex.method_proofs WHERE session_id IN (SELECT id FROM convex.sessions WHERE token_hash=$1)")
        .bind(sec::digest(&token))
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE convex.sessions SET revoked_at=now() WHERE token_hash=$1")
            .bind(sec::digest(&token))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(with_session(
        &context,
        "",
        MessageResponse {
            message: "Signed out.".into(),
        },
    ))
}

pub async fn change_password(
    context: Context,
    credentials: Credentials,
    req: PasswordChange,
) -> AppResult<SessionUpdate<MessageResponse>> {
    storage::rate_limit(
        &context.pool,
        &format!(
            "password:{}",
            sec::digest(&sec::session_token(&credentials)?)
        ),
    )
    .await?;
    let new_hash = sec::hash_password(req.new_password).await?;
    let mut tx = transaction(&context.pool).await?;
    let user = authenticate(&mut tx, &credentials).await?;
    crate::postgres::policy::ensure_local_allowed(&mut tx, user.id).await?;
    let hash: String = sqlx::query_scalar(
        "SELECT password_hash FROM convex.local_credentials WHERE user_id=$1 FOR UPDATE",
    )
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(AppError::unauthorized)?;
    sec::verify_password(req.current_password, hash).await?;
    replace_password(&mut tx, user.id, new_hash).await?;
    let token = storage::new_session(&mut tx, user.id, sec::LoginMethod::Local).await?;
    tx.commit().await?;
    Ok(with_session(
        &context,
        &token,
        MessageResponse {
            message: "Password changed. Other sessions have been signed out.".into(),
        },
    ))
}

async fn replace_password(
    tx: &mut Transaction<'_, Postgres>,
    user: UserId,
    hash: String,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE convex.local_credentials SET password_hash=$2,changed_at=now() WHERE user_id=$1",
    )
    .bind(user)
    .bind(hash)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE convex.recovery_tokens SET consumed_at=now() WHERE user_id=$1 AND consumed_at IS NULL")
        .bind(user)
        .execute(&mut **tx)
        .await?;
    revoke_all(tx, user).await
}

pub async fn recovery_token(tx: &mut Transaction<'_, Postgres>, user: UserId) -> AppResult<String> {
    crate::postgres::policy::ensure_local_allowed(tx, user).await?;
    let token = sec::token();
    sqlx::query("UPDATE convex.recovery_tokens SET consumed_at=now() WHERE user_id=$1 AND consumed_at IS NULL")
        .bind(user)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO convex.recovery_tokens (id,user_id,token_hash) VALUES ($1,$2,$3)")
        .bind(RecoveryTokenId::new())
        .bind(user)
        .bind(sec::digest(&token))
        .execute(&mut **tx)
        .await?;
    Ok(token)
}

pub async fn request_recovery(
    context: Context,
    req: RecoveryRequest,
) -> AppResult<MessageResponse> {
    let email = sec::email(&req.email)?;
    storage::rate_limit(&context.pool, &format!("recovery:{email}")).await?;
    if context.security.mail.is_some() {
        // Run lookup and delivery outside the response path, so mailbox existence and
        // SMTP latency cannot be distinguished through this endpoint's response time.
        tokio::spawn(async move {
            let delivery = async {
                let mut tx = transaction(&context.pool)
        .await?;
                if let Some(user) = sqlx::query_scalar::<_,UserId>("SELECT u.id
             FROM convex.users u
             JOIN convex.local_credentials c ON u.id=c.user_id
             CROSS JOIN convex.authentication_policy p
             WHERE c.email=$1
             AND u.disabled_at IS NULL
             AND (NOT p.oidc_required
             OR u.is_instance_admin)
             FOR UPDATE OF u,c")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await? {
                    let token = recovery_token(&mut tx, user)
        .await?;
                    tx.commit()
        .await?;
                    let message = format!(
                        "Reset your password within 30 minutes:\n{}/auth/reset#{}\nIf you did not request this, ignore this email.",
                        context.security.origin, token,
                    );
                    context.security.send(&email, "Reset your Convex password", message).await?;
                }
                Ok::<_, AppError>(())
            }.await;
            if let Err(error) = delivery {
                tracing::error!(%error,"Recovery email delivery failed");
            }
        });
    }
    Ok(MessageResponse { message:"If recovery email is available for this account, a reset link will be sent. Otherwise, contact your installation operator.".into() })
}

pub async fn reset_password(
    context: Context,
    req: ResetRequest,
) -> AppResult<SessionUpdate<MessageResponse>> {
    storage::rate_limit(&context.pool, &format!("reset:{}", sec::digest(&req.token))).await?;
    let hash = sec::hash_password(req.password).await?;
    let mut tx = transaction(&context.pool).await?;
    // Lock the user before its recovery rows: password changes and token issuance use the same order.
    let user = sqlx::query_scalar::<_, UserId>(
        "SELECT u.id
             FROM convex.users u
             JOIN convex.recovery_tokens r ON r.user_id=u.id
             WHERE r.token_hash=$1
             AND u.disabled_at IS NULL
             FOR UPDATE OF u",
    )
    .bind(sec::digest(&req.token))
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::validation("This reset link is invalid or expired."))?;
    crate::postgres::policy::ensure_local_allowed(&mut tx, user).await?;
    let valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1
             FROM convex.recovery_tokens
             WHERE token_hash=$1
             AND consumed_at IS NULL
             AND expires_at > now())",
    )
    .bind(sec::digest(&req.token))
    .fetch_one(&mut *tx)
    .await?;
    if !valid {
        return Err(AppError::validation(
            "This reset link is invalid or expired.",
        ));
    }
    replace_password(&mut tx, user, hash).await?;
    tx.commit().await?;
    Ok(with_session(
        &context,
        "",
        MessageResponse {
            message: "Password reset. Sign in with your new password.".into(),
        },
    ))
}

pub async fn sessions(context: Context, credentials: Credentials) -> AppResult<Vec<SessionInfo>> {
    let mut tx = transaction(&context.pool).await?;
    let user = authenticate(&mut tx, &credentials).await?;
    let list = sqlx::query_as(
        "SELECT id,created_at,expires_at,last_activity_at,token_hash=$2 AS current
             FROM convex.sessions
             WHERE user_id=$1
             AND revoked_at IS NULL
             AND expires_at > now()
             ORDER BY created_at,id",
    )
    .bind(user.id)
    .bind(sec::digest(&sec::session_token(&credentials)?))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(list)
}

pub async fn revoke_session(
    context: Context,
    credentials: Credentials,
    req: RevokeSession,
) -> AppResult<MessageResponse> {
    let mut tx = transaction(&context.pool).await?;
    let user = authenticate(&mut tx, &credentials).await?;
    let result =
        sqlx::query("UPDATE convex.sessions SET revoked_at=now() WHERE id=$1 AND user_id=$2")
            .bind(req.session_id)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found());
    }
    sqlx::query("DELETE FROM convex.oidc_flows WHERE session_id=$1")
        .bind(req.session_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM convex.method_proofs WHERE session_id=$1")
        .bind(req.session_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(MessageResponse {
        message: "Session signed out.".into(),
    })
}

pub async fn admin_accounts(
    context: Context,
    credentials: Credentials,
) -> AppResult<Vec<AdminAccount>> {
    let mut tx = transaction(&context.pool).await?;
    let actor = authenticate(&mut tx, &credentials).await?;
    if !actor.is_instance_admin {
        return Err(AppError::forbidden());
    }
    let accounts=sqlx::query_as("SELECT u.id,u.display_name,c.email,u.disabled_at IS NOT NULL AS disabled,u.is_instance_admin
             FROM convex.users u
             LEFT JOIN convex.local_credentials c ON c.user_id=u.id
             ORDER BY u.created_at,u.id")
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(accounts)
}

/// Operator-only recovery. Database access is the authorization boundary.
pub async fn recover_account(
    pool: &sqlx::PgPool,
    email: &str,
    enable: bool,
) -> anyhow::Result<String> {
    use anyhow::Context as _;
    let email = sec::email(email)?;
    let mut tx = transaction(pool).await?;
    let (user, disabled): (UserId, bool) = sqlx::query_as("SELECT u.id,u.disabled_at IS NOT NULL
             FROM convex.users u
             JOIN convex.local_credentials c ON c.user_id=u.id
             WHERE c.email=$1
             FOR UPDATE OF u,c")
        .bind(email)
        .fetch_optional(&mut *tx)
        .await?.context("No existing local credential for that email. OIDC-only accounts recover through their provider.")?;
    anyhow::ensure!(
        !disabled || enable,
        "Account is disabled. Add --enable to explicitly restore account access."
    );
    if enable {
        sqlx::query("UPDATE convex.users SET disabled_at=NULL,updated_at=now() WHERE id=$1")
            .bind(user)
            .execute(&mut *tx)
            .await?;
    }
    revoke_all(&mut tx, user).await?;
    let token = recovery_token(&mut tx, user).await?;
    tx.commit().await?;
    Ok(token)
}
