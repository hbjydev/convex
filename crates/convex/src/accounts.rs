use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    http::{ApiJson, AppError, AppResult, AppState},
    security as sec, workspaces,
};

#[derive(Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub display_name: String,
    pub is_instance_admin: bool,
}
#[derive(Serialize, ToSchema)]
pub struct BootstrapInfo {
    pub setup_required: bool,
    pub setup_enabled: bool,
    pub mail_available: bool,
    pub oidc_display_name: Option<String>,
    pub oidc_required: bool,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetupRequest {
    pub setup_token: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub workspace_name: String,
    pub workspace_slug: String,
    pub team_name: String,
    pub team_key: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}
#[derive(Serialize, ToSchema)]
pub struct AccountResponse {
    pub user: User,
    pub workspaces: Vec<workspaces::Workspace>,
}
#[derive(Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PasswordChange {
    pub current_password: String,
    pub new_password: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoveryRequest {
    pub email: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResetRequest {
    pub token: String,
    pub password: String,
}
#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct SessionInfo {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub current: bool,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RevokeSession {
    pub session_id: Uuid,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DisableRequest {
    pub user_id: Uuid,
    pub disabled: bool,
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
    headers: &HeaderMap,
) -> AppResult<User> {
    let hash = sec::digest(&sec::session_token(headers)?);
    let user = sqlx::query_as::<_, User>("SELECT u.id,u.display_name,u.is_instance_admin FROM convex.users u JOIN convex.sessions s ON s.user_id=u.id CROSS JOIN convex.authentication_policy p LEFT JOIN convex.oidc_identities i ON i.id=s.oidc_identity_id AND i.user_id=u.id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>now() AND u.disabled_at IS NULL AND ((s.login_method='local' AND (NOT p.oidc_required OR u.is_instance_admin) AND EXISTS(SELECT 1 FROM convex.local_credentials c WHERE c.user_id=u.id)) OR (s.login_method='oidc' AND p.oidc_enabled AND s.provider_revision=p.provider_revision AND i.unlinked_at IS NULL AND i.issuer=p.oidc_issuer)) FOR UPDATE OF u,s").bind(&hash).fetch_optional(&mut **tx).await?.ok_or_else(AppError::unauthorized)?;
    sqlx::query("UPDATE convex.sessions SET last_activity_at=now() WHERE token_hash=$1")
        .bind(hash)
        .execute(&mut **tx)
        .await?;
    Ok(user)
}

pub async fn revoke_all(tx: &mut Transaction<'_, Postgres>, user: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE convex.sessions SET revoked_at=now() WHERE user_id=$1 AND revoked_at IS NULL",
    )
    .bind(user)
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM convex.oidc_flows WHERE user_id=$1 OR session_id IN (SELECT id FROM convex.sessions WHERE user_id=$1)")
        .bind(user).execute(&mut **tx).await?;
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
        id: Uuid::new_v4(),
        display_name,
        is_instance_admin: admin,
    };
    sqlx::query("INSERT INTO convex.users (id,display_name,is_instance_admin) VALUES ($1,$2,$3)")
        .bind(user.id)
        .bind(&user.display_name)
        .bind(admin)
        .execute(&mut **tx)
        .await?;
    crate::auth_policy::ensure_local_allowed(tx, user.id).await?;
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

pub fn signed_in(state: &AppState, token: &str, body: impl Serialize) -> Response {
    let mut response = Json(body).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, state.security.cookie(token));
    response
}

#[utoipa::path(get, path="/api/v1/auth/bootstrap", operation_id="getBootstrap", tag="Accounts", responses((status=200,body=BootstrapInfo),(status=503,body=crate::http::ApiError)))]
pub async fn bootstrap(State(state): State<AppState>) -> AppResult<Json<BootstrapInfo>> {
    let initialized: bool =
        sqlx::query_scalar("SELECT initialized_at IS NOT NULL FROM convex.installation")
            .fetch_one(&state.pool)
            .await?;
    Ok(Json(BootstrapInfo {
        setup_required: !initialized,
        setup_enabled: !initialized && state.security.setup_token.is_some(),
        mail_available: state.security.mail.is_some(),
        oidc_display_name: state
            .security
            .authentication
            .provider
            .as_ref()
            .filter(|p| p.enabled)
            .map(|p| p.display_name.clone()),
        oidc_required: state.security.authentication.oidc_required,
    }))
}

#[utoipa::path(post, path="/api/v1/auth/setup", operation_id="setupInstance", tag="Accounts", request_body=SetupRequest, responses((status=200,body=User),(status=400,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn setup(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<SetupRequest>,
) -> AppResult<Response> {
    sec::rate_limit(&state.pool, "setup").await?;
    let expected = state
        .security
        .setup_token
        .as_ref()
        .ok_or_else(AppError::forbidden)?;
    // Fixed-length digest comparison avoids leaking setup credential prefixes.
    if sec::digest(expected)
        .bytes()
        .zip(sec::digest(&req.setup_token).bytes())
        .fold(0u8, |a, (b, c)| a | (b ^ c))
        != 0
    {
        return Err(AppError::forbidden());
    }
    let email = sec::email(&req.email)?;
    let display_name = sec::name(&req.display_name)?;
    let workspace_name = sec::name(&req.workspace_name)?;
    let workspace_slug = sec::workspace_slug(&req.workspace_slug)?;
    let team_name = sec::name(&req.team_name)?;
    let team_key = sec::team_key(&req.team_key)?;
    let hash = sec::hash_password(req.password).await?;
    let mut tx = transaction(&state.pool).await?;
    let initialized: bool =
        sqlx::query_scalar("SELECT initialized_at IS NOT NULL FROM convex.installation FOR UPDATE")
            .fetch_one(&mut *tx)
            .await?;
    if initialized {
        return Err(AppError::conflict("This instance has already been set up."));
    }
    let user = create_local(&mut tx, email, hash, display_name, true).await?;
    workspaces::create_workspace(
        &mut tx,
        user.id,
        workspace_name,
        workspace_slug,
        team_name,
        team_key,
    )
    .await?;
    sqlx::query("UPDATE convex.installation SET initialized_at=now()")
        .execute(&mut *tx)
        .await?;
    let token = sec::new_session(&mut tx, user.id, sec::LoginMethod::Local).await?;
    tx.commit().await?;
    Ok(signed_in(&state, &token, user))
}

#[utoipa::path(post, path="/api/v1/auth/login", operation_id="login", tag="Accounts", request_body=LoginRequest, responses((status=200,body=User),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<LoginRequest>,
) -> AppResult<Response> {
    let email = sec::email(&req.email)?;
    sec::rate_limit(&state.pool, &format!("login:{email}")).await?;
    let mut tx = transaction(&state.pool).await?;
    let record = sqlx::query_as::<_, (Uuid,String,Option<DateTime<Utc>>)>("SELECT u.id,c.password_hash,u.disabled_at FROM convex.users u JOIN convex.local_credentials c ON c.user_id=u.id WHERE c.email=$1 FOR UPDATE OF u,c").bind(email).fetch_optional(&mut *tx).await?;
    let dummy = state.dummy_hash.clone();
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
    if let Ok(old) = sec::session_token(&headers) {
        sqlx::query("UPDATE convex.sessions SET revoked_at=now() WHERE token_hash=$1")
            .bind(sec::digest(&old))
            .execute(&mut *tx)
            .await?;
    }
    let token = sec::new_session(&mut tx, id, sec::LoginMethod::Local).await?;
    let user = sqlx::query_as::<_, User>(
        "SELECT id,display_name,is_instance_admin FROM convex.users WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(signed_in(&state, &token, user))
}

#[utoipa::path(get, path="/api/v1/auth/me", operation_id="getAccount", tag="Accounts", security(("cookieSession"=[])), responses((status=200,body=AccountResponse),(status=401,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AccountResponse>> {
    let mut tx = transaction(&state.pool).await?;
    let user = authenticate(&mut tx, &headers).await?;
    let workspaces = workspaces::list_for_user(&mut tx, user.id).await?;
    tx.commit().await?;
    Ok(Json(AccountResponse { user, workspaces }))
}

#[utoipa::path(post, path="/api/v1/auth/logout", operation_id="logout", tag="Accounts", responses((status=200,body=MessageResponse),(status=403,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    if let Ok(token) = sec::session_token(&headers) {
        let mut tx = transaction(&state.pool).await?;
        sqlx::query("DELETE FROM convex.oidc_flows WHERE session_id IN (SELECT id FROM convex.sessions WHERE token_hash=$1)")
            .bind(sec::digest(&token)).execute(&mut *tx).await?;
        sqlx::query("UPDATE convex.sessions SET revoked_at=now() WHERE token_hash=$1")
            .bind(sec::digest(&token))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(signed_in(
        &state,
        "",
        MessageResponse {
            message: "Signed out.".into(),
        },
    ))
}

#[utoipa::path(post, path="/api/v1/auth/password", operation_id="changePassword", tag="Accounts", security(("cookieSession"=[])), request_body=PasswordChange, responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<PasswordChange>,
) -> AppResult<Response> {
    sec::rate_limit(
        &state.pool,
        &format!("password:{}", sec::digest(&sec::session_token(&headers)?)),
    )
    .await?;
    let new_hash = sec::hash_password(req.new_password).await?;
    let mut tx = transaction(&state.pool).await?;
    let user = authenticate(&mut tx, &headers).await?;
    crate::auth_policy::ensure_local_allowed(&mut tx, user.id).await?;
    let hash: String = sqlx::query_scalar(
        "SELECT password_hash FROM convex.local_credentials WHERE user_id=$1 FOR UPDATE",
    )
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(AppError::unauthorized)?;
    sec::verify_password(req.current_password, hash).await?;
    replace_password(&mut tx, user.id, new_hash).await?;
    let token = sec::new_session(&mut tx, user.id, sec::LoginMethod::Local).await?;
    tx.commit().await?;
    Ok(signed_in(
        &state,
        &token,
        MessageResponse {
            message: "Password changed. Other sessions have been signed out.".into(),
        },
    ))
}

async fn replace_password(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    hash: String,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE convex.local_credentials SET password_hash=$2,changed_at=now() WHERE user_id=$1",
    )
    .bind(user)
    .bind(hash)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE convex.recovery_tokens SET consumed_at=now() WHERE user_id=$1 AND consumed_at IS NULL").bind(user).execute(&mut **tx).await?;
    revoke_all(tx, user).await
}

pub async fn recovery_token(tx: &mut Transaction<'_, Postgres>, user: Uuid) -> AppResult<String> {
    crate::auth_policy::ensure_local_allowed(tx, user).await?;
    let token = sec::token();
    sqlx::query("UPDATE convex.recovery_tokens SET consumed_at=now() WHERE user_id=$1 AND consumed_at IS NULL").bind(user).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO convex.recovery_tokens (id,user_id,token_hash) VALUES ($1,$2,$3)")
        .bind(Uuid::new_v4())
        .bind(user)
        .bind(sec::digest(&token))
        .execute(&mut **tx)
        .await?;
    Ok(token)
}

#[utoipa::path(post, path="/api/v1/auth/recovery", operation_id="requestRecovery", tag="Accounts", request_body=RecoveryRequest, responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn request_recovery(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<RecoveryRequest>,
) -> AppResult<Json<MessageResponse>> {
    let email = sec::email(&req.email)?;
    sec::rate_limit(&state.pool, &format!("recovery:{email}")).await?;
    if state.security.mail.is_some() {
        // Run lookup and delivery outside the response path, so mailbox existence and
        // SMTP latency cannot be distinguished through this endpoint's response time.
        tokio::spawn(async move {
            let delivery = async {
                let mut tx = transaction(&state.pool).await?;
                if let Some(user) = sqlx::query_scalar::<_,Uuid>("SELECT u.id FROM convex.users u JOIN convex.local_credentials c ON u.id=c.user_id CROSS JOIN convex.authentication_policy p WHERE c.email=$1 AND u.disabled_at IS NULL AND (NOT p.oidc_required OR u.is_instance_admin) FOR UPDATE OF u,c").bind(&email).fetch_optional(&mut *tx).await? {
                    let token = recovery_token(&mut tx,user).await?;
                    tx.commit().await?;
                    state.security.send(&email,"Reset your Convex password",format!("Reset your password within 30 minutes:\n{}/auth/reset#{}\nIf you did not request this, ignore this email.",state.security.origin,token)).await?;
                }
                Ok::<_,AppError>(())
            }.await;
            if let Err(error) = delivery {
                tracing::error!(%error,"Recovery email delivery failed");
            }
        });
    }
    Ok(Json(MessageResponse { message:"If recovery email is available for this account, a reset link will be sent. Otherwise, contact your installation operator.".into() }))
}

#[utoipa::path(post, path="/api/v1/auth/reset", operation_id="resetPassword", tag="Accounts", request_body=ResetRequest, responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn reset_password(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ResetRequest>,
) -> AppResult<Response> {
    sec::rate_limit(&state.pool, &format!("reset:{}", sec::digest(&req.token))).await?;
    let hash = sec::hash_password(req.password).await?;
    let mut tx = transaction(&state.pool).await?;
    // Lock the user before its recovery rows: password changes and token issuance use the same order.
    let user = sqlx::query_scalar::<_,Uuid>("SELECT u.id FROM convex.users u JOIN convex.recovery_tokens r ON r.user_id=u.id WHERE r.token_hash=$1 AND u.disabled_at IS NULL FOR UPDATE OF u").bind(sec::digest(&req.token)).fetch_optional(&mut *tx).await?.ok_or_else(||AppError::validation("This reset link is invalid or expired."))?;
    crate::auth_policy::ensure_local_allowed(&mut tx, user).await?;
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.recovery_tokens WHERE token_hash=$1 AND consumed_at IS NULL AND expires_at > now())").bind(sec::digest(&req.token)).fetch_one(&mut *tx).await?;
    if !valid {
        return Err(AppError::validation(
            "This reset link is invalid or expired.",
        ));
    }
    replace_password(&mut tx, user, hash).await?;
    tx.commit().await?;
    Ok(signed_in(
        &state,
        "",
        MessageResponse {
            message: "Password reset. Sign in with your new password.".into(),
        },
    ))
}

#[utoipa::path(get, path="/api/v1/auth/sessions", operation_id="listSessions", tag="Accounts", security(("cookieSession"=[])), responses((status=200,body=Vec<SessionInfo>),(status=401,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<SessionInfo>>> {
    let mut tx = transaction(&state.pool).await?;
    let user = authenticate(&mut tx, &headers).await?;
    let list = sqlx::query_as("SELECT id,created_at,expires_at,last_activity_at,token_hash=$2 AS current FROM convex.sessions WHERE user_id=$1 AND revoked_at IS NULL AND expires_at > now() ORDER BY created_at,id").bind(user.id).bind(sec::digest(&sec::session_token(&headers)?)).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(list))
}

#[utoipa::path(post, path="/api/v1/auth/sessions/revoke", operation_id="revokeSession", tag="Accounts", security(("cookieSession"=[])), request_body=RevokeSession, responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn revoke_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<RevokeSession>,
) -> AppResult<Json<MessageResponse>> {
    let mut tx = transaction(&state.pool).await?;
    let user = authenticate(&mut tx, &headers).await?;
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
    tx.commit().await?;
    Ok(Json(MessageResponse {
        message: "Session signed out.".into(),
    }))
}

#[utoipa::path(post, path="/api/v1/admin/accounts/disable", operation_id="setAccountDisabled", tag="Accounts", security(("cookieSession"=[])), request_body=DisableRequest, responses((status=200,body=MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn disable(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<DisableRequest>,
) -> AppResult<Json<MessageResponse>> {
    let mut tx = transaction(&state.pool).await?;
    // Serializes installation-wide administration before acquiring account locks.
    sqlx::query("SELECT singleton FROM convex.installation FOR UPDATE")
        .execute(&mut *tx)
        .await?;
    let actor = authenticate(&mut tx, &headers).await?;
    if !actor.is_instance_admin {
        return Err(AppError::forbidden());
    }
    if req.user_id == actor.id && req.disabled {
        return Err(AppError::conflict("You cannot disable your own account."));
    }
    // Team operations lock workspace rows too; an account cannot lose its final active ownership.
    let spaces: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM convex.workspaces ORDER BY id FOR UPDATE")
            .fetch_all(&mut *tx)
            .await?;
    let target = sqlx::query_as::<_, User>(
        "SELECT id,display_name,is_instance_admin FROM convex.users WHERE id=$1 FOR UPDATE",
    )
    .bind(req.user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(AppError::not_found)?;
    if req.disabled {
        if target.is_instance_admin {
            let admins: i64 = sqlx::query_scalar("SELECT count(*) FROM convex.users WHERE is_instance_admin AND disabled_at IS NULL AND id<>$1").bind(target.id).fetch_one(&mut *tx).await?;
            if admins == 0 {
                return Err(AppError::conflict("Keep an active instance administrator."));
            }
            let recovery_admins: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM convex.local_credentials WHERE user_id=$1) OR EXISTS(SELECT 1 FROM convex.users u JOIN convex.local_credentials c ON c.user_id=u.id WHERE u.id<>$1 AND u.is_instance_admin AND u.disabled_at IS NULL)")
                .bind(target.id).fetch_one(&mut *tx).await?;
            if !recovery_admins {
                return Err(AppError::conflict(
                    "Keep an enabled instance administrator with local credentials for recovery.",
                ));
            }
        }
        for space in spaces {
            workspaces::protect_last_owner(&mut tx, space, target.id).await?;
        }
    }
    sqlx::query("UPDATE convex.users SET disabled_at=CASE WHEN $2 THEN now() ELSE NULL END,updated_at=now() WHERE id=$1").bind(target.id).bind(req.disabled).execute(&mut *tx).await?;
    revoke_all(&mut tx, target.id).await?;
    tx.commit().await?;
    Ok(Json(MessageResponse {
        message: if req.disabled {
            "Account disabled. All sessions revoked."
        } else {
            "Account enabled. Sign in again to continue."
        }
        .into(),
    }))
}

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct AdminAccount {
    pub id: Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub disabled: bool,
    pub is_instance_admin: bool,
}
#[utoipa::path(get,path="/api/v1/admin/accounts",operation_id="listAccounts",tag="Accounts",security(("cookieSession"=[])),responses((status=200,body=Vec<AdminAccount>),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn admin_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AdminAccount>>> {
    let mut tx = transaction(&state.pool).await?;
    let actor = authenticate(&mut tx, &headers).await?;
    if !actor.is_instance_admin {
        return Err(AppError::forbidden());
    }
    let accounts=sqlx::query_as("SELECT u.id,u.display_name,c.email,u.disabled_at IS NOT NULL AS disabled,u.is_instance_admin FROM convex.users u LEFT JOIN convex.local_credentials c ON c.user_id=u.id ORDER BY u.created_at,u.id").fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(accounts))
}
