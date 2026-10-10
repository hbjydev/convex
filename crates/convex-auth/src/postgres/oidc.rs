use crate::{
    credentials::{Authorization, Credentials, OidcCompletion},
    methods_model,
    oidc_model::StartRequest,
    postgres::{Context, accounts, methods, policy, security as storage},
    security as sec,
};
use chrono::{DateTime, Utc};
use convex_core::{Error as AppError, Result as AppResult};
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, HttpRequest,
    HttpResponse, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

pub fn binding(credentials: &Credentials) -> Option<String> {
    credentials.browser.clone()
}

const CALLBACK: &str = "/api/v1/auth/oidc/callback";
const MAX_RESPONSE: usize = 1024 * 1024;

#[derive(Default)]
pub struct Runtime {
    metadata: Mutex<Option<(Instant, CoreProviderMetadata, Option<Instant>)>>,
}

// Discovery, keys and exchange all use this bounded transport. Redirects are
// rejected, including discovery redirects, and upstream errors are never logged.
async fn fetch(request: HttpRequest) -> Result<HttpResponse, std::io::Error> {
    let unavailable = || std::io::Error::other("OIDC transport failed");
    let url = reqwest::Url::parse(&request.uri().to_string()).map_err(|_| unavailable())?;
    if !safe_url(&url) {
        return Err(unavailable());
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(3))
        .build()
        .map_err(|_| unavailable())?;
    let (parts, body) = request.into_parts();
    let mut upstream = client
        .request(parts.method, url)
        .headers(parts.headers)
        .body(body)
        .send()
        .await
        .map_err(|_| unavailable())?;
    if upstream
        .content_length()
        .is_some_and(|n| n > MAX_RESPONSE as u64)
    {
        return Err(unavailable());
    }
    let mut response = http::Response::builder().status(upstream.status());
    *response.headers_mut().ok_or_else(unavailable)? = upstream.headers().clone();
    let mut body = Vec::new();
    while let Some(chunk) = upstream.chunk().await.map_err(|_| unavailable())? {
        if body.len() + chunk.len() > MAX_RESPONSE {
            return Err(unavailable());
        }
        body.extend_from_slice(&chunk);
    }
    response.body(body).map_err(|_| unavailable())
}
fn safe_url(url: &reqwest::Url) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && (url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
}
fn unavailable() -> AppError {
    AppError::internal("OIDC provider authentication unavailable")
}
impl Runtime {
    async fn metadata(&self, issuer: &str, refresh: bool) -> AppResult<CoreProviderMetadata> {
        let mut cache = self.metadata.lock().await;
        if let Some((time, metadata, forced)) = cache.as_ref()
            && ((!refresh && time.elapsed() < Duration::from_secs(300))
                || (refresh && forced.is_some_and(|t| t.elapsed() < Duration::from_secs(30))))
        {
            return Ok(metadata.clone());
        }
        if refresh && let Some((_, _, forced)) = cache.as_mut() {
            *forced = Some(Instant::now());
        }
        let metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(issuer.into()).map_err(|_| unavailable())?,
            &fetch,
        )
        .await
        .map_err(|_| unavailable())?;
        if metadata.issuer().as_str() != issuer
            || !safe_url(metadata.authorization_endpoint().url())
            || metadata
                .token_endpoint()
                .is_none_or(|url| !safe_url(url.url()))
        {
            return Err(unavailable());
        }
        let forced = if refresh {
            Some(Instant::now())
        } else {
            cache.as_ref().and_then(|(_, _, forced)| *forced)
        };
        *cache = Some((Instant::now(), metadata.clone(), forced));
        Ok(metadata)
    }
}

// Provider credentials determine the exchange revision. Each flow separately
// stores the exact public origin so callbacks cannot cross differently configured origins.
fn revision(context: &Context) -> AppResult<String> {
    context
        .security
        .authentication
        .exchange_revision()
        .ok_or_else(AppError::forbidden)
}

pub async fn start(
    context: Context,
    credentials: Credentials,
    req: StartRequest,
) -> AppResult<Authorization> {
    begin(
        context,
        credentials,
        FlowKind::Login {
            invitation_token: req.invitation_token,
        },
    )
    .await
}

pub enum FlowKind {
    Login {
        invitation_token: Option<String>,
    },
    Link {
        user: Uuid,
        session: Uuid,
        verified_at: DateTime<Utc>,
    },
    Reauth {
        user: Uuid,
        session: Uuid,
        action: methods_model::MethodAction,
    },
}
pub async fn begin(
    context: Context,
    credentials: Credentials,
    kind: FlowKind,
) -> AppResult<Authorization> {
    let config = context
        .security
        .authentication
        .provider
        .as_ref()
        .filter(|p| p.enabled)
        .ok_or_else(AppError::forbidden)?;
    let browser = binding(&credentials).unwrap_or_else(sec::token);
    storage::rate_limit(
        &context.pool,
        &format!("oidc-start:{}", sec::digest(&browser)),
    )
    .await?;
    let revision = revision(&context)?;
    let metadata = context.oidc.metadata(&config.issuer, false).await?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}{CALLBACK}", context.security.origin))
            .map_err(|_| unavailable())?,
    );
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let authorization = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("profile".into()))
        .set_pkce_challenge(challenge);
    let authorization = if matches!(&kind, FlowKind::Login { .. }) {
        authorization
    } else {
        authorization
            .set_max_age(Duration::ZERO)
            .add_prompt(openidconnect::core::CoreAuthPrompt::Login)
    };
    let (url, csrf, nonce) = authorization.url();
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    let initialized: bool =
        sqlx::query_scalar("SELECT initialized_at IS NOT NULL FROM convex.installation")
            .fetch_one(&mut *tx)
            .await?;
    if !initialized {
        return Err(AppError::forbidden());
    }
    let source = if let Ok(token) = sec::session_token(&credentials) {
        match accounts::authenticate(&mut tx, &credentials).await {
            Ok(user) => {
                let id: Uuid =
                    sqlx::query_scalar("SELECT id FROM convex.sessions WHERE token_hash=$1")
                        .bind(sec::digest(&token))
                        .fetch_one(&mut *tx)
                        .await?;
                Some((user.id, id))
            }
            Err(AppError::Unauthorized) => None,
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    let (intent, invitation, action, verified_at) = match kind {
        FlowKind::Login { invitation_token } => {
            let invitation = if let Some(token) = invitation_token {
                Some(context.invitations.valid_token(&mut tx, &token).await?.id)
            } else {
                None
            };
            (
                if invitation.is_some() {
                    "invite"
                } else {
                    "login"
                },
                invitation,
                None,
                None,
            )
        }
        FlowKind::Link {
            user,
            session,
            verified_at,
        } => {
            if source != Some((user, session))
                || verified_at < Utc::now() - chrono::Duration::minutes(5)
            {
                return Err(AppError::unauthorized());
            }
            policy::ensure_local_allowed(&mut tx, user).await?;
            ("link", None, None, Some(verified_at))
        }
        FlowKind::Reauth {
            user,
            session,
            action,
        } => {
            if source != Some((user, session)) {
                return Err(AppError::unauthorized());
            }
            methods::enabled_identity(&mut tx, user).await?;
            ("reauth", None, Some(action.as_str()), None)
        }
    };
    sqlx::query("DELETE FROM convex.oidc_flows WHERE expires_at <= now()")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO convex.oidc_flows
             (id,state_hash,browser_hash,intent,exchange_revision,nonce,pkce_verifier,invitation_id,public_origin,user_id,session_id,method_action,local_verified_at,expires_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,CASE WHEN $4 IN ( 'link' , 'reauth' ) THEN
             LEAST(now() + interval '5 minutes' ,COALESCE($13,now()) + interval '5 minutes' ) ELSE now() +
             interval '10 minutes' END)")
        .bind(Uuid::new_v4())
        .bind(sec::digest(csrf.secret()))
        .bind(sec::digest(&browser))
        .bind(intent)
        .bind(revision)
        .bind(nonce.secret())
        .bind(verifier.secret())
        .bind(invitation)
        .bind(&context.security.origin)
        .bind(source.map(|(u,_)|u))
        .bind(source.map(|(_,s)|s))
        .bind(action)
        .bind(verified_at)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Authorization {
        authorization_url: url.into(),
        browser_token: browser,
    })
}

struct CallbackQuery {
    state: String,
    code: Option<String>,
}
fn callback_query(raw: &str) -> AppResult<CallbackQuery> {
    if raw.len() > 16384 {
        return Err(AppError::validation(
            "Invalid provider callback. Restart sign-in.",
        ));
    }
    let url = reqwest::Url::parse(&format!("http://localhost/?{raw}"))
        .map_err(|_| AppError::unauthorized())?;
    let mut state = None;
    let mut code = None;
    let mut error = None;
    for (key, value) in url.query_pairs() {
        let slot = match key.as_ref() {
            "state" => &mut state,
            "code" => &mut code,
            "error" => &mut error,
            _ => continue,
        };
        if slot.is_some() || value.is_empty() {
            return Err(AppError::unauthorized());
        }
        *slot = Some(value.into_owned());
    }
    if code.is_some() == error.is_some() {
        return Err(AppError::unauthorized());
    }
    Ok(CallbackQuery {
        state: state.ok_or_else(AppError::unauthorized)?,
        code,
    })
}
#[derive(sqlx::FromRow)]
pub struct Flow {
    pub method_action: Option<String>,
    local_verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    pub browser_hash: String,
    pub exchange_revision: String,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    id: Uuid,
    intent: String,
    nonce: Option<String>,
    pkce_verifier: Option<String>,
    invitation_id: Option<Uuid>,
}

pub async fn complete(
    context: &Context,
    credentials: &Credentials,
    raw: &str,
) -> AppResult<OidcCompletion> {
    let query = callback_query(raw)?;
    let browser = binding(credentials).ok_or_else(AppError::unauthorized)?;
    let revision = revision(context)?;
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    let flow:  Flow  =  sqlx::query_as("SELECT
             id,intent,nonce,pkce_verifier,invitation_id,user_id,session_id,method_action,local_verified_at,created_at,browser_hash,exchange_revision
             FROM convex.oidc_flows
             WHERE state_hash=$1
             AND browser_hash=$2
             AND exchange_revision=$3
             AND consumed_at IS NULL
             AND expires_at>now()
             AND intent IN ( 'login' , 'invite' , 'link' , 'reauth' )
             AND public_origin=$4
             FOR UPDATE")
        .bind(sec::digest(&query.state))
        .bind(sec::digest(&browser))
        .bind(&revision)
        .bind(&context.security.origin)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(AppError::unauthorized)?;
    let sensitive = matches!(flow.intent.as_str(), "link" | "reauth");
    if sensitive && (flow.user_id.is_none() || flow.session_id.is_none()) {
        return Err(AppError::unauthorized());
    }
    if flow.intent == "link"
        && flow.local_verified_at.is_none_or(|at| {
            at < Utc::now() - chrono::Duration::minutes(5)
                || at > Utc::now() + chrono::Duration::seconds(30)
        })
    {
        return Err(AppError::unauthorized());
    }
    if flow.intent == "reauth" {
        methods_model::MethodAction::parse(
            flow.method_action
                .as_deref()
                .ok_or_else(AppError::unauthorized)?,
        )?;
    }
    sqlx::query(
        "UPDATE convex.oidc_flows SET consumed_at=now(),nonce=NULL,pkce_verifier=NULL WHERE id=$1",
    )
    .bind(flow.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?; // Burn before remote work; failed exchanges cannot be replayed.
    let code = query.code.ok_or_else(AppError::unauthorized)?;
    let config = context
        .security
        .authentication
        .provider
        .as_ref()
        .filter(|p| p.enabled)
        .ok_or_else(AppError::forbidden)?;
    let metadata = context.oidc.metadata(&config.issuer, false).await?;
    let mut client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}{CALLBACK}", context.security.origin))
            .map_err(|_| unavailable())?,
    );
    let tokens = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|_| unavailable())?
        .set_pkce_verifier(PkceCodeVerifier::new(
            flow.pkce_verifier
                .clone()
                .ok_or_else(AppError::unauthorized)?,
        ))
        .request_async(&fetch)
        .await
        .map_err(|_| unavailable())?;
    let id_token = tokens.id_token().ok_or_else(AppError::unauthorized)?;
    let nonce = Nonce::new(flow.nonce.clone().ok_or_else(AppError::unauthorized)?);
    let unknown_key = matches!(
        id_token.claims(&client.id_token_verifier(), &nonce),
        Err(
            openidconnect::ClaimsVerificationError::SignatureVerification(
                openidconnect::SignatureVerificationError::NoMatchingKey
            )
        )
    );
    if unknown_key {
        // Refresh only on an unknown signing key, at most once per 30 seconds.
        // Revalidate the same token; never repeat a code exchange.
        let metadata = context.oidc.metadata(&config.issuer, true).await?;
        client = CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("{}{CALLBACK}", context.security.origin))
                .map_err(|_| unavailable())?,
        );
    }
    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &nonce)
        .map_err(|_| AppError::unauthorized())?;
    if claims.audiences().len() != 1
        || claims
            .authorized_party()
            .is_some_and(|p| p.as_str() != config.client_id)
        || claims.issue_time() > Utc::now() + chrono::Duration::seconds(30)
        || claims.subject().as_str().is_empty()
    {
        return Err(AppError::unauthorized());
    }
    if sensitive
        && claims.auth_time().is_none_or(|time| {
            time < flow.created_at - chrono::Duration::seconds(30)
                || time < Utc::now() - chrono::Duration::minutes(5)
                || time > Utc::now() + chrono::Duration::seconds(30)
        })
    {
        return Err(AppError::unauthorized());
    }
    if let Some(expected) = claims.access_token_hash() {
        let actual = AccessTokenHash::from_token(
            tokens.access_token(),
            id_token
                .signing_alg()
                .map_err(|_| AppError::unauthorized())?,
            id_token
                .signing_key(&verifier)
                .map_err(|_| AppError::unauthorized())?,
        )
        .map_err(|_| AppError::unauthorized())?;
        if actual != *expected {
            return Err(AppError::unauthorized());
        }
    }
    let subject = claims.subject().as_str();
    let name = claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.as_str())
        .and_then(|n| sec::name(n).ok())
        .unwrap_or_else(|| "Workspace member".into());
    let mut tx = accounts::transaction(&context.pool).await?;
    context.check_policy().await?;
    // A configuration transition during exchange deletes the flow. Recheck its
    // continued presence and exact revision before applying any domain changes.
    let live: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1
             FROM convex.oidc_flows
             WHERE id=$1
             AND exchange_revision=$2
             AND consumed_at IS NOT NULL
             AND expires_at>now())",
    )
    .bind(flow.id)
    .bind(&revision)
    .fetch_one(&mut *tx)
    .await?;
    if !live {
        return Err(AppError::unauthorized());
    }
    if let Some(source) = flow.session_id {
        let actor = accounts::authenticate(&mut tx, credentials).await?;
        let session: Uuid =
            sqlx::query_scalar("SELECT id FROM convex.sessions WHERE token_hash=$1")
                .bind(sec::digest(&sec::session_token(credentials)?))
                .fetch_one(&mut *tx)
                .await?;
        if session != source || Some(actor.id) != flow.user_id {
            return Err(AppError::unauthorized());
        }
    }
    if flow.intent == "link" {
        let user = flow.user_id.ok_or_else(AppError::unauthorized)?;
        policy::ensure_local_allowed(&mut tx, user).await?;
        let identity = methods::link_identity(&mut tx, user, &config.issuer, subject).await?;
        accounts::revoke_all(&mut tx, user).await?;
        let token = storage::new_session(&mut tx, user, sec::LoginMethod::Oidc(identity)).await?;
        tx.commit().await?;
        return Ok(OidcCompletion::Linked(token));
    }
    if flow.intent == "reauth" {
        let user = flow.user_id.ok_or_else(AppError::unauthorized)?;
        let identity = methods::enabled_identity(&mut tx, user).await?;
        let matches: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM convex.oidc_identities WHERE id=$1 AND issuer=$2 AND subject=$3)")
        .bind(identity)
        .bind(&config.issuer)
        .bind(subject)
        .fetch_one(&mut *tx)
        .await?;
        if !matches {
            return Err(AppError::unauthorized());
        }
        let token = methods::issue_proof(
            &mut tx,
            &flow,
            identity,
            claims.auth_time().ok_or_else(AppError::unauthorized)?,
        )
        .await?;
        sqlx::query("DELETE FROM convex.oidc_flows WHERE id=$1")
            .bind(flow.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(OidcCompletion::Proof(token));
    }
    let invitation = if flow.intent == "invite" {
        Some(
            context
                .invitations
                .valid_id(
                    &mut tx,
                    flow.invitation_id.ok_or_else(AppError::unauthorized)?,
                )
                .await?,
        )
    } else {
        None
    };
    let identity:  Option<(Uuid,Uuid,bool)>  =  sqlx::query_as("SELECT id,user_id,unlinked_at IS NULL FROM convex.oidc_identities WHERE issuer=$1 AND subject=$2 FOR UPDATE")
        .bind(&config.issuer)
        .bind(subject)
        .fetch_optional(&mut *tx)
        .await?;
    let (identity, user) = match identity {
        Some((identity, user, true)) => (identity, user),
        Some(_) => return Err(AppError::unauthorized()),
        None => {
            if invitation.is_none() {
                return Err(AppError::forbidden());
            }
            let user = Uuid::new_v4();
            let identity = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO convex.users (id,display_name,is_instance_admin) VALUES ($1,$2,false)",
            )
            .bind(user)
            .bind(name)
            .execute(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO convex.oidc_identities (id,user_id,issuer,subject) VALUES ($1,$2,$3,$4)")
        .bind(identity)
        .bind(user)
        .bind(&config.issuer)
        .bind(subject)
        .execute(&mut *tx)
        .await?;
            (identity, user)
        }
    };
    if let Some(invitation) = invitation {
        context
            .invitations
            .accept(&mut tx, &invitation, user)
            .await?;
        accounts::revoke_all(&mut tx, user).await?;
    }
    // Rotate the presented browser session without revoking other devices.
    if let Ok(old) = sec::session_token(credentials) {
        sqlx::query("DELETE FROM convex.oidc_flows WHERE session_id IN (SELECT id FROM convex.sessions WHERE token_hash=$1)")
        .bind(sec::digest(&old))
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM convex.method_proofs WHERE session_id IN (SELECT id FROM convex.sessions WHERE token_hash=$1)")
        .bind(sec::digest(&old))
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE convex.sessions SET revoked_at=now() WHERE token_hash=$1 AND revoked_at IS NULL")
        .bind(sec::digest(&old))
        .execute(&mut *tx)
        .await?;
    }
    let token = storage::new_session(&mut tx, user, sec::LoginMethod::Oidc(identity)).await?;
    sqlx::query("DELETE FROM convex.oidc_flows WHERE id=$1")
        .bind(flow.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(OidcCompletion::Login(token))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callbacks_reject_ambiguous_parameters_and_unsafe_endpoints() {
        for raw in [
            "",
            "code=x",
            "state=x&code=y&error=z",
            "state=x&state=y&code=z",
            "state=x&code=",
            "state=x&error=a&error=b",
        ] {
            assert!(callback_query(raw).is_err());
        }
        assert!(callback_query("state=x&code=y&session_state=z").is_ok());
        assert!(
            callback_query("state=x&error=access_denied")
                .unwrap()
                .code
                .is_none()
        );
        for url in [
            "http://example.com/token",
            "https://user:pass@example.com",
            "https://example.com/#fragment",
        ] {
            assert!(!safe_url(&reqwest::Url::parse(url).unwrap()));
        }
        assert!(safe_url(
            &reqwest::Url::parse("http://127.0.0.1:8081/token").unwrap()
        ));
    }
}
