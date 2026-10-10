use crate::{
    accounts::models::MessageResponse,
    credentials::{Authorization, Credentials, SessionUpdate},
    methods_model::*,
    postgres::{Context, accounts, oidc, policy, security as storage},
    security as sec,
};
use chrono::{DateTime, Utc};
use convex_core::{Error as AppError, Result as AppResult};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub async fn session_id(
    tx: &mut Transaction<'_, Postgres>,
    credentials: &Credentials,
) -> AppResult<Uuid> {
    Ok(
        sqlx::query_scalar("SELECT id FROM convex.sessions WHERE token_hash=$1")
            .bind(sec::digest(&sec::session_token(credentials)?))
            .fetch_one(&mut **tx)
            .await?,
    )
}
async fn throttle(context: &Context, credentials: &Credentials) -> AppResult<()> {
    storage::rate_limit(
        &context.pool,
        &format!(
            "method-change:{}",
            sec::digest(&sec::session_token(credentials)?)
        ),
    )
    .await
}
async fn local_proof(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    password: String,
) -> AppResult<()> {
    policy::ensure_local_allowed(tx, user).await?;
    let hash: String =
        sqlx::query_scalar("SELECT password_hash FROM convex.local_credentials WHERE user_id=$1")
            .bind(user)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(AppError::unauthorized)?;
    sec::verify_password(password, hash).await
}
pub async fn enabled_identity(tx: &mut Transaction<'_, Postgres>, user: Uuid) -> AppResult<Uuid> {
    sqlx::query_scalar(
        "SELECT i.id
             FROM convex.oidc_identities i
             CROSS JOIN convex.authentication_policy p
             WHERE i.user_id=$1
             AND i.unlinked_at IS NULL
             AND p.oidc_enabled
             AND i.issuer=p.oidc_issuer",
    )
    .bind(user)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        AppError::conflict("Keep an enabled single sign-on method before removing your password.")
    })
}
async fn protect_recovery(
    tx: &mut Transaction<'_, Postgres>,
    user: &crate::accounts::models::User,
) -> AppResult<()> {
    if user.is_instance_admin {
        let another: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1
             FROM convex.users u
             JOIN convex.local_credentials c ON c.user_id=u.id
             WHERE u.id<>$1
             AND u.is_instance_admin
             AND u.disabled_at IS NULL)",
        )
        .bind(user.id)
        .fetch_one(&mut **tx)
        .await?;
        if !another {
            return Err(AppError::conflict(
                "Keep an enabled instance administrator with local credentials for recovery.",
            ));
        }
    }
    Ok(())
}

// This query also validates proof status for the UI. A proof is never a session:
// the caller must first authenticate the exact session that created it.
async fn proof(
    tx: &mut Transaction<'_, Postgres>,
    credentials: &Credentials,
    user: Uuid,
    consume: Option<MethodAction>,
) -> AppResult<Option<(Uuid, MethodAction)>> {
    let Ok(token) = credentials
        .method_proof
        .clone()
        .ok_or_else(AppError::unauthorized)
    else {
        return Ok(None);
    };
    let Some(browser) = oidc::binding(credentials) else {
        return Ok(None);
    };
    let source = session_id(tx, credentials).await?;
    let row: Option<(Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT f.id,f.oidc_identity_id,f.action
             FROM convex.method_proofs f
             JOIN convex.oidc_identities i ON i.id=f.oidc_identity_id
             CROSS JOIN convex.authentication_policy p
             WHERE f.token_hash=$1
             AND f.user_id=$2
             AND f.session_id=$3
             AND f.browser_hash=$4
             AND f.expires_at>now()
             AND f.exchange_revision=p.exchange_revision
             AND p.oidc_enabled
             AND i.issuer=p.oidc_issuer
             AND i.unlinked_at IS NULL
             FOR UPDATE OF f",
    )
    .bind(sec::digest(&token))
    .bind(user)
    .bind(source)
    .bind(sec::digest(&browser))
    .fetch_optional(&mut **tx)
    .await?;
    let Some((id, identity, action)) = row else {
        return Ok(None);
    };
    let action = MethodAction::parse(&action)?;
    if let Some(expected) = consume {
        if action != expected {
            return Err(AppError::unauthorized());
        }
        sqlx::query("DELETE FROM convex.method_proofs WHERE id=$1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(Some((identity, action)))
}

pub async fn list(context: Context, credentials: Credentials) -> AppResult<Methods> {
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &credentials).await?;
    let local_email: Option<String> =
        sqlx::query_scalar("SELECT email FROM convex.local_credentials WHERE user_id=$1")
            .bind(user.id)
            .fetch_optional(&mut *tx)
            .await?;
    let local_usable = local_email.is_some()
        && (!context.security.authentication.oidc_required || user.is_instance_admin);
    let oidc: Vec<OidcMethod> = sqlx::query_as(
        "SELECT i.id,i.issuer,(p.oidc_enabled
             AND i.issuer=p.oidc_issuer) AS usable
             FROM convex.oidc_identities i
             CROSS JOIN convex.authentication_policy p
             WHERE i.user_id=$1
             AND i.unlinked_at IS NULL
             ORDER BY i.created_at,i.id",
    )
    .bind(user.id)
    .fetch_all(&mut *tx)
    .await?;
    let enabled = oidc.iter().any(|i| i.usable);
    let recovery_allowed = !user.is_instance_admin
        || sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1
             FROM convex.users u
             JOIN convex.local_credentials c ON c.user_id=u.id
             WHERE u.id<>$1
             AND u.is_instance_admin
             AND u.disabled_at IS NULL)",
        )
        .bind(user.id)
        .fetch_one(&mut *tx)
        .await?;
    let pending_action = proof(&mut tx, &credentials, user.id, None)
        .await?
        .map(|(_, action)| action);
    let response = Methods {
        local_email: local_email.clone(),
        local_usable,
        oidc,
        provider_display_name: context
            .security
            .authentication
            .provider
            .as_ref()
            .filter(|p| p.enabled)
            .map(|p| p.display_name.clone()),
        can_link_oidc: local_usable
            && !enabled
            && context
                .security
                .authentication
                .provider
                .as_ref()
                .is_some_and(|p| p.enabled),
        can_add_local: local_email.is_none()
            && enabled
            && (!context.security.authentication.oidc_required || user.is_instance_admin),
        can_remove_local: local_email.is_some() && enabled && recovery_allowed,
        pending_action,
    };
    tx.commit().await?;
    Ok(response)
}

pub async fn link(
    context: Context,
    credentials: Credentials,
    req: LocalProof,
) -> AppResult<Authorization> {
    throttle(&context, &credentials).await?;
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &credentials).await?;
    local_proof(&mut tx, user.id, req.current_password).await?;
    let source = session_id(&mut tx, &credentials).await?;
    let verified_at = Utc::now();
    tx.commit().await?;
    oidc::begin(
        context,
        credentials,
        oidc::FlowKind::Link {
            user: user.id,
            session: source,
            verified_at,
        },
    )
    .await
}

pub async fn prove(
    context: Context,
    credentials: Credentials,
    req: ProveOidc,
) -> AppResult<Authorization> {
    throttle(&context, &credentials).await?;
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &credentials).await?;
    enabled_identity(&mut tx, user.id).await?;
    match req.action {
        MethodAction::AddLocal => {
            policy::ensure_local_allowed(&mut tx, user.id).await?;
            let local: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM convex.local_credentials WHERE user_id=$1)",
            )
            .bind(user.id)
            .fetch_one(&mut *tx)
            .await?;
            if local {
                return Err(AppError::conflict(
                    "A password is already attached to your account.",
                ));
            }
        }
        MethodAction::RemoveLocal => {
            protect_recovery(&mut tx, &user).await?;
        }
    }
    let source = session_id(&mut tx, &credentials).await?;
    tx.commit().await?;
    oidc::begin(
        context,
        credentials,
        oidc::FlowKind::Reauth {
            user: user.id,
            session: source,
            action: req.action,
        },
    )
    .await
}
pub async fn link_identity(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    issuer: &str,
    subject: &str,
) -> AppResult<Uuid> {
    let identity: Option<(Uuid,Uuid,bool)> = sqlx::query_as("SELECT id,user_id,unlinked_at IS NULL FROM convex.oidc_identities WHERE issuer=$1 AND subject=$2 FOR UPDATE")
        .bind(issuer)
        .bind(subject)
        .fetch_optional(&mut **tx)
        .await?;
    if identity
        .as_ref()
        .is_some_and(|(_, owner, _)| *owner != user)
    {
        return Err(AppError::conflict(
            "This provider identity belongs to another account. Accounts cannot be merged.",
        ));
    }
    let attached: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.oidc_identities WHERE user_id=$1 AND issuer=$2 AND unlinked_at IS NULL)")
        .bind(user)
        .bind(issuer)
        .fetch_one(&mut **tx)
        .await?;
    if attached {
        return Err(AppError::conflict(
            "A provider identity is already attached to your account.",
        ));
    }
    let id = if let Some((id, _, _)) = identity {
        sqlx::query("UPDATE convex.oidc_identities SET unlinked_at=NULL WHERE id=$1")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO convex.oidc_identities (id,user_id,issuer,subject) VALUES ($1,$2,$3,$4)",
        )
        .bind(id)
        .bind(user)
        .bind(issuer)
        .bind(subject)
        .execute(&mut **tx)
        .await?;
        id
    };
    Ok(id)
}
pub async fn issue_proof(
    tx: &mut Transaction<'_, Postgres>,
    flow: &oidc::Flow,
    identity: Uuid,
    verified_at: DateTime<Utc>,
) -> AppResult<String> {
    let token = sec::token();
    sqlx::query("DELETE FROM convex.method_proofs WHERE expires_at<=now() OR (user_id=$1 AND session_id=$2 AND action=$3)")
        .bind(flow.user_id)
        .bind(flow.session_id)
        .bind(flow.method_action.as_deref())
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO convex.method_proofs
             (id,token_hash,user_id,session_id,oidc_identity_id,action,browser_hash,exchange_revision,verified_at,expires_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,LEAST(now()+interval '5 minutes' ,$9+interval '5 minutes' ))")
        .bind(Uuid::new_v4())
        .bind(sec::digest(&token))
        .bind(flow.user_id)
        .bind(flow.session_id)
        .bind(identity)
        .bind(flow.method_action.as_deref())
        .bind(&flow.browser_hash)
        .bind(&flow.exchange_revision)
        .bind(verified_at)
        .execute(&mut **tx)
        .await?;
    Ok(token)
}

pub async fn add_local(
    context: Context,
    credentials: Credentials,
    req: AddLocal,
) -> AppResult<SessionUpdate<MessageResponse>> {
    throttle(&context, &credentials).await?;
    let email = sec::email(&req.email)?;
    let hash = sec::hash_password(req.password).await?;
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &credentials).await?;
    policy::ensure_local_allowed(&mut tx, user.id).await?;
    proof(&mut tx, &credentials, user.id, Some(MethodAction::AddLocal))
        .await?
        .ok_or_else(AppError::unauthorized)?;
    sqlx::query(
        "INSERT INTO convex.local_credentials (user_id,email,password_hash) VALUES ($1,$2,$3)",
    )
    .bind(user.id)
    .bind(email)
    .bind(hash)
    .execute(&mut *tx)
    .await?;
    accounts::revoke_all(&mut tx, user.id).await?;
    let token = storage::new_session(&mut tx, user.id, sec::LoginMethod::Local).await?;
    rotate(
        &context,
        tx,
        token,
        "Password added. Other sessions have been signed out.",
    )
    .await
}

pub async fn remove_local(
    context: Context,
    credentials: Credentials,
) -> AppResult<SessionUpdate<MessageResponse>> {
    throttle(&context, &credentials).await?;
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &credentials).await?;
    protect_recovery(&mut tx, &user).await?;
    let (identity, _) = proof(
        &mut tx,
        &credentials,
        user.id,
        Some(MethodAction::RemoveLocal),
    )
    .await?
    .ok_or_else(AppError::unauthorized)?;
    sqlx::query("DELETE FROM convex.recovery_tokens WHERE user_id=$1")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    if sqlx::query("DELETE FROM convex.local_credentials WHERE user_id=$1")
        .bind(user.id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        == 0
    {
        return Err(AppError::conflict(
            "No password is attached to your account.",
        ));
    }
    accounts::revoke_all(&mut tx, user.id).await?;
    let token = storage::new_session(&mut tx, user.id, sec::LoginMethod::Oidc(identity)).await?;
    rotate(
        &context,
        tx,
        token,
        "Password removed. Other sessions have been signed out.",
    )
    .await
}

pub async fn remove_oidc(
    context: Context,
    credentials: Credentials,
    req: RemoveOidc,
) -> AppResult<SessionUpdate<MessageResponse>> {
    throttle(&context, &credentials).await?;
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &credentials).await?;
    local_proof(&mut tx, user.id, req.current_password).await?;
    let changed = sqlx::query(
        "UPDATE convex.oidc_identities SET unlinked_at=now()
         WHERE id=$1 AND user_id=$2 AND unlinked_at IS NULL",
    )
    .bind(req.identity_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(AppError::not_found());
    }
    accounts::revoke_all(&mut tx, user.id).await?;
    let token = storage::new_session(&mut tx, user.id, sec::LoginMethod::Local).await?;
    rotate(
        &context,
        tx,
        token,
        "Single sign-on removed. Other sessions have been signed out.",
    )
    .await
}

async fn rotate(
    _state: &Context,
    tx: Transaction<'_, Postgres>,
    token: String,
    message: &str,
) -> AppResult<SessionUpdate<MessageResponse>> {
    tx.commit().await?;
    let mut result = SessionUpdate::new(
        &token,
        MessageResponse {
            message: message.into(),
        },
    );
    result.clear_method_proof = true;
    Ok(result)
}
