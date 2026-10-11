use crate::{credentials::Credentials, security::*};
use convex_core::{Error as AppError, Result as AppResult};
use convex_ids::{OidcIdentityId, SessionId, UserId};
use sqlx::{PgPool, Postgres, Transaction};

pub async fn rate_limit(pool: &PgPool, subject: &str) -> AppResult<()> {
    for (bucket, maximum) in [("global".to_owned(), 120), (digest(subject), 10)] {
        let attempts:  i32  =  sqlx::query_scalar("INSERT INTO convex.auth_rate_limits (bucket)
             VALUES ($1)
             ON CONFLICT (bucket) DO UPDATE
             SET attempts = CASE WHEN convex.auth_rate_limits.started_at < now() - interval '15 minutes' THEN 1
             ELSE convex.auth_rate_limits.attempts + 1 END, started_at = CASE WHEN
             convex.auth_rate_limits.started_at < now() - interval '15 minutes' THEN now() ELSE
             convex.auth_rate_limits.started_at END
             RETURNING attempts")
        .bind(bucket)
        .fetch_one(pool)
        .await?;
        if attempts > maximum {
            return Err(AppError::rate_limited());
        }
    }
    sqlx::query("DELETE FROM convex.auth_rate_limits WHERE started_at < now() - interval '1 day'")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn session_method(
    tx: &mut Transaction<'_, Postgres>,
    credentials: &Credentials,
) -> AppResult<LoginMethod> {
    let identity: Option<OidcIdentityId> =
        sqlx::query_scalar("SELECT oidc_identity_id FROM convex.sessions WHERE token_hash=$1")
            .bind(digest(&session_token(credentials)?))
            .fetch_one(&mut **tx)
            .await?;
    Ok(identity.map_or(LoginMethod::Local, LoginMethod::Oidc))
}

pub async fn new_session(
    tx: &mut Transaction<'_, Postgres>,
    user: UserId,
    method: LoginMethod,
) -> AppResult<String> {
    let enabled: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM convex.users WHERE id=$1 AND disabled_at IS NULL)",
    )
    .bind(user)
    .fetch_one(&mut **tx)
    .await?;
    if !enabled {
        return Err(AppError::unauthorized());
    }
    let identity = match method {
        LoginMethod::Local => {
            crate::postgres::policy::ensure_local_allowed(tx, user).await?;
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM convex.local_credentials WHERE user_id=$1)",
            )
            .bind(user)
            .fetch_one(&mut **tx)
            .await?;
            if !exists {
                return Err(AppError::unauthorized());
            }
            None
        }
        LoginMethod::Oidc(identity) => {
            let eligible: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1
             FROM convex.oidc_identities i
             CROSS JOIN convex.authentication_policy p
             WHERE i.id=$1
             AND i.user_id=$2
             AND i.unlinked_at IS NULL
             AND p.oidc_enabled
             AND i.issuer=p.oidc_issuer)",
            )
            .bind(identity)
            .bind(user)
            .fetch_one(&mut **tx)
            .await?;
            if !eligible {
                return Err(AppError::unauthorized());
            }
            Some(identity)
        }
    };
    let token = token();
    sqlx::query("INSERT INTO convex.sessions
             (id,user_id,token_hash,login_method,oidc_identity_id,provider_revision,expires_at) SELECT
             $1,$2,$3,$4,$5,CASE WHEN $5::uuid IS NOT NULL THEN provider_revision END,now() + make_interval(secs
             => session_ttl_seconds::double precision)
             FROM convex.authentication_policy")
        .bind(SessionId::new())
        .bind(user)
        .bind(digest(&token))
        .bind(if identity.is_some() { "oidc" } else { "local" })
        .bind(identity)
        .execute(&mut **tx)
        .await?;
    Ok(token)
}
