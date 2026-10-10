//! Account method changes require a live session and a freshly proven remaining method.
use crate::{
    accounts, auth_policy,
    http::{ApiJson, AppError, AppResult, AppState},
    oidc, security as sec,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderValue, header::SET_COOKIE},
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MethodAction {
    AddLocal,
    RemoveLocal,
}
impl MethodAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AddLocal => "add_local",
            Self::RemoveLocal => "remove_local",
        }
    }
    pub(crate) fn parse(value: &str) -> AppResult<Self> {
        match value {
            "add_local" => Ok(Self::AddLocal),
            "remove_local" => Ok(Self::RemoveLocal),
            _ => Err(AppError::unauthorized()),
        }
    }
}
#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct OidcMethod {
    pub id: Uuid,
    pub issuer: String,
    pub usable: bool,
}
#[derive(Serialize, ToSchema)]
pub struct Methods {
    pub local_email: Option<String>,
    pub local_usable: bool,
    pub oidc: Vec<OidcMethod>,
    pub provider_display_name: Option<String>,
    pub can_link_oidc: bool,
    pub can_add_local: bool,
    pub can_remove_local: bool,
    pub pending_action: Option<MethodAction>,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalProof {
    pub current_password: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProveOidc {
    pub action: MethodAction,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddLocal {
    pub email: String,
    pub password: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveOidc {
    pub identity_id: Uuid,
    pub current_password: String,
}

pub(crate) async fn session_id(
    tx: &mut Transaction<'_, Postgres>,
    headers: &HeaderMap,
) -> AppResult<Uuid> {
    Ok(
        sqlx::query_scalar("SELECT id FROM convex.sessions WHERE token_hash=$1")
            .bind(sec::digest(&sec::session_token(headers)?))
            .fetch_one(&mut **tx)
            .await?,
    )
}
async fn throttle(state: &AppState, headers: &HeaderMap) -> AppResult<()> {
    sec::rate_limit(
        &state.pool,
        &format!(
            "method-change:{}",
            sec::digest(&sec::session_token(headers)?)
        ),
    )
    .await
}
async fn local_proof(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    password: String,
) -> AppResult<()> {
    auth_policy::ensure_local_allowed(tx, user).await?;
    let hash: String =
        sqlx::query_scalar("SELECT password_hash FROM convex.local_credentials WHERE user_id=$1")
            .bind(user)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(AppError::unauthorized)?;
    sec::verify_password(password, hash).await
}
pub(crate) async fn enabled_identity(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
) -> AppResult<Uuid> {
    sqlx::query_scalar("SELECT i.id FROM convex.oidc_identities i CROSS JOIN convex.authentication_policy p WHERE i.user_id=$1 AND i.unlinked_at IS NULL AND p.oidc_enabled AND i.issuer=p.oidc_issuer")
        .bind(user).fetch_optional(&mut **tx).await?.ok_or_else(||AppError::conflict("Keep an enabled single sign-on method before removing your password."))
}
async fn protect_recovery(
    tx: &mut Transaction<'_, Postgres>,
    user: &accounts::User,
) -> AppResult<()> {
    if user.is_instance_admin {
        let another:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.users u JOIN convex.local_credentials c ON c.user_id=u.id WHERE u.id<>$1 AND u.is_instance_admin AND u.disabled_at IS NULL)").bind(user.id).fetch_one(&mut **tx).await?;
        if !another {
            return Err(AppError::conflict(
                "Keep an enabled instance administrator with local credentials for recovery.",
            ));
        }
    }
    Ok(())
}
fn proof_token(headers: &HeaderMap) -> AppResult<String> {
    let mut values = headers
        .get_all("cookie")
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(';'))
        .filter_map(|v| v.trim().strip_prefix("convex_method_proof="));
    let value = values.next().ok_or_else(AppError::unauthorized)?;
    if values.next().is_some() || value.len() != 64 || !value.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(AppError::unauthorized());
    }
    Ok(value.into())
}
pub(crate) fn proof_cookie(state: &AppState, token: &str) -> HeaderValue {
    format!(
        "convex_method_proof={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        if token.is_empty() { 0 } else { 300 },
        if state.security.secure_cookies {
            "; Secure"
        } else {
            ""
        }
    )
    .parse()
    .expect("generated cookie")
}
// This query also validates proof status for the UI. A proof is never a session:
// the caller must first authenticate the exact session that created it.
async fn proof(
    tx: &mut Transaction<'_, Postgres>,
    headers: &HeaderMap,
    user: Uuid,
    consume: Option<MethodAction>,
) -> AppResult<Option<(Uuid, MethodAction)>> {
    let Ok(token) = proof_token(headers) else {
        return Ok(None);
    };
    let Some(browser) = oidc::binding(headers) else {
        return Ok(None);
    };
    let source = session_id(tx, headers).await?;
    let row:Option<(Uuid,Uuid,String)>=sqlx::query_as("SELECT f.id,f.oidc_identity_id,f.action FROM convex.method_proofs f JOIN convex.oidc_identities i ON i.id=f.oidc_identity_id CROSS JOIN convex.authentication_policy p WHERE f.token_hash=$1 AND f.user_id=$2 AND f.session_id=$3 AND f.browser_hash=$4 AND f.expires_at>now() AND f.exchange_revision=p.exchange_revision AND p.oidc_enabled AND i.issuer=p.oidc_issuer AND i.unlinked_at IS NULL FOR UPDATE OF f")
        .bind(sec::digest(&token)).bind(user).bind(source).bind(sec::digest(&browser)).fetch_optional(&mut **tx).await?;
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
#[utoipa::path(get,path="/api/v1/auth/methods",operation_id="getLoginMethods",tag="Accounts",security(("cookieSession"=[])),responses((status=200,body=Methods),(status=401,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn list(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Methods>> {
    let mut tx = accounts::transaction(&state.pool).await?;
    state.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    let local_email: Option<String> =
        sqlx::query_scalar("SELECT email FROM convex.local_credentials WHERE user_id=$1")
            .bind(user.id)
            .fetch_optional(&mut *tx)
            .await?;
    let local_usable = local_email.is_some()
        && (!state.security.authentication.oidc_required || user.is_instance_admin);
    let oidc:Vec<OidcMethod>=sqlx::query_as("SELECT i.id,i.issuer,(p.oidc_enabled AND i.issuer=p.oidc_issuer) AS usable FROM convex.oidc_identities i CROSS JOIN convex.authentication_policy p WHERE i.user_id=$1 AND i.unlinked_at IS NULL ORDER BY i.created_at,i.id").bind(user.id).fetch_all(&mut *tx).await?;
    let enabled = oidc.iter().any(|i| i.usable);
    let recovery_allowed=!user.is_instance_admin || sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM convex.users u JOIN convex.local_credentials c ON c.user_id=u.id WHERE u.id<>$1 AND u.is_instance_admin AND u.disabled_at IS NULL)").bind(user.id).fetch_one(&mut *tx).await?;
    let pending_action = proof(&mut tx, &headers, user.id, None)
        .await?
        .map(|(_, action)| action);
    let response = Methods {
        local_email: local_email.clone(),
        local_usable,
        oidc,
        provider_display_name: state
            .security
            .authentication
            .provider
            .as_ref()
            .filter(|p| p.enabled)
            .map(|p| p.display_name.clone()),
        can_link_oidc: local_usable
            && !enabled
            && state
                .security
                .authentication
                .provider
                .as_ref()
                .is_some_and(|p| p.enabled),
        can_add_local: local_email.is_none()
            && enabled
            && (!state.security.authentication.oidc_required || user.is_instance_admin),
        can_remove_local: local_email.is_some() && enabled && recovery_allowed,
        pending_action,
    };
    tx.commit().await?;
    Ok(Json(response))
}
#[utoipa::path(post,path="/api/v1/auth/methods/oidc/link",operation_id="linkOidcMethod",tag="Accounts",security(("cookieSession"=[])),request_body=LocalProof,responses((status=200,body=oidc::StartResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn link(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<LocalProof>,
) -> AppResult<Response> {
    throttle(&state, &headers).await?;
    let mut tx = accounts::transaction(&state.pool).await?;
    state.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    local_proof(&mut tx, user.id, req.current_password).await?;
    let source = session_id(&mut tx, &headers).await?;
    let verified_at = Utc::now();
    tx.commit().await?;
    oidc::begin(
        state,
        headers,
        oidc::FlowKind::Link {
            user: user.id,
            session: source,
            verified_at,
        },
    )
    .await
}
#[utoipa::path(post,path="/api/v1/auth/methods/oidc/prove",operation_id="proveOidcMethod",tag="Accounts",security(("cookieSession"=[])),request_body=ProveOidc,responses((status=200,body=oidc::StartResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn prove(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<ProveOidc>,
) -> AppResult<Response> {
    throttle(&state, &headers).await?;
    let mut tx = accounts::transaction(&state.pool).await?;
    state.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    enabled_identity(&mut tx, user.id).await?;
    match req.action {
        MethodAction::AddLocal => {
            auth_policy::ensure_local_allowed(&mut tx, user.id).await?;
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
    let source = session_id(&mut tx, &headers).await?;
    tx.commit().await?;
    oidc::begin(
        state,
        headers,
        oidc::FlowKind::Reauth {
            user: user.id,
            session: source,
            action: req.action,
        },
    )
    .await
}
pub(crate) async fn link_identity(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    issuer: &str,
    subject: &str,
) -> AppResult<Uuid> {
    let identity:Option<(Uuid,Uuid,bool)>=sqlx::query_as("SELECT id,user_id,unlinked_at IS NULL FROM convex.oidc_identities WHERE issuer=$1 AND subject=$2 FOR UPDATE").bind(issuer).bind(subject).fetch_optional(&mut **tx).await?;
    if identity
        .as_ref()
        .is_some_and(|(_, owner, _)| *owner != user)
    {
        return Err(AppError::conflict(
            "This provider identity belongs to another account. Accounts cannot be merged.",
        ));
    }
    let attached:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.oidc_identities WHERE user_id=$1 AND issuer=$2 AND unlinked_at IS NULL)").bind(user).bind(issuer).fetch_one(&mut **tx).await?;
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
pub(crate) async fn issue_proof(
    tx: &mut Transaction<'_, Postgres>,
    flow: &oidc::Flow,
    identity: Uuid,
    verified_at: DateTime<Utc>,
) -> AppResult<String> {
    let token = sec::token();
    sqlx::query("DELETE FROM convex.method_proofs WHERE expires_at<=now() OR (user_id=$1 AND session_id=$2 AND action=$3)").bind(flow.user_id).bind(flow.session_id).bind(flow.method_action.as_deref()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO convex.method_proofs (id,token_hash,user_id,session_id,oidc_identity_id,action,browser_hash,exchange_revision,verified_at,expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,LEAST(now()+interval '5 minutes',$9+interval '5 minutes'))").bind(Uuid::new_v4()).bind(sec::digest(&token)).bind(flow.user_id).bind(flow.session_id).bind(identity).bind(flow.method_action.as_deref()).bind(&flow.browser_hash).bind(&flow.exchange_revision).bind(verified_at).execute(&mut **tx).await?;
    Ok(token)
}
async fn rotate(
    state: &AppState,
    tx: Transaction<'_, Postgres>,
    token: String,
    message: &str,
) -> AppResult<Response> {
    tx.commit().await?;
    let mut response = accounts::signed_in(
        state,
        &token,
        accounts::MessageResponse {
            message: message.into(),
        },
    );
    response
        .headers_mut()
        .append(SET_COOKIE, proof_cookie(state, ""));
    Ok(response)
}
#[utoipa::path(post,path="/api/v1/auth/methods/local/add",operation_id="addLocalMethod",tag="Accounts",security(("cookieSession"=[])),request_body=AddLocal,responses((status=200,body=accounts::MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn add_local(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<AddLocal>,
) -> AppResult<Response> {
    throttle(&state, &headers).await?;
    let email = sec::email(&req.email)?;
    let hash = sec::hash_password(req.password).await?;
    let mut tx = accounts::transaction(&state.pool).await?;
    state.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    auth_policy::ensure_local_allowed(&mut tx, user.id).await?;
    proof(&mut tx, &headers, user.id, Some(MethodAction::AddLocal))
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
    let token = sec::new_session(&mut tx, user.id, sec::LoginMethod::Local).await?;
    rotate(
        &state,
        tx,
        token,
        "Password added. Other sessions have been signed out.",
    )
    .await
}
#[utoipa::path(post,path="/api/v1/auth/methods/local/remove",operation_id="removeLocalMethod",tag="Accounts",security(("cookieSession"=[])),responses((status=200,body=accounts::MessageResponse),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn remove_local(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Response> {
    throttle(&state, &headers).await?;
    let mut tx = accounts::transaction(&state.pool).await?;
    state.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    protect_recovery(&mut tx, &user).await?;
    let (identity, _) = proof(&mut tx, &headers, user.id, Some(MethodAction::RemoveLocal))
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
    let token = sec::new_session(&mut tx, user.id, sec::LoginMethod::Oidc(identity)).await?;
    rotate(
        &state,
        tx,
        token,
        "Password removed. Other sessions have been signed out.",
    )
    .await
}
#[utoipa::path(post,path="/api/v1/auth/methods/oidc/remove",operation_id="removeOidcMethod",tag="Accounts",security(("cookieSession"=[])),request_body=RemoveOidc,responses((status=200,body=accounts::MessageResponse),(status=400,body=crate::http::ApiError),(status=401,body=crate::http::ApiError),(status=403,body=crate::http::ApiError),(status=404,body=crate::http::ApiError),(status=409,body=crate::http::ApiError),(status=429,body=crate::http::ApiError),(status=503,body=crate::http::ApiError)))]
pub async fn remove_oidc(
    State(state): State<AppState>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<RemoveOidc>,
) -> AppResult<Response> {
    throttle(&state, &headers).await?;
    let mut tx = accounts::transaction(&state.pool).await?;
    state.check_policy().await?;
    let user = accounts::authenticate(&mut tx, &headers).await?;
    local_proof(&mut tx, user.id, req.current_password).await?;
    if sqlx::query("UPDATE convex.oidc_identities SET unlinked_at=now() WHERE id=$1 AND user_id=$2 AND unlinked_at IS NULL").bind(req.identity_id).bind(user.id).execute(&mut *tx).await?.rows_affected()==0{return Err(AppError::not_found());}
    accounts::revoke_all(&mut tx, user.id).await?;
    let token = sec::new_session(&mut tx, user.id, sec::LoginMethod::Local).await?;
    rotate(
        &state,
        tx,
        token,
        "Single sign-on removed. Other sessions have been signed out.",
    )
    .await
}
