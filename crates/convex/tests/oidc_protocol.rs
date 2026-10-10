//! A signed, in-process provider for deterministic callback and atomicity cases.
use super::*;
use axum::{
    Json,
    extract::{Form, State},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use openidconnect::{
    Audience, ClientId, EmptyAdditionalClaims, EndUserEmail, IssuerUrl, JsonWebKeyId, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, PrivateSigningKey, StandardClaims, SubjectIdentifier,
    core::{CoreIdToken, CoreIdTokenClaims, CoreJwsSigningAlgorithm, CoreRsaPrivateSigningKey},
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tokio::sync::{Mutex, Notify};

#[derive(Clone)]
struct Code {
    nonce: String,
    challenge: String,
    mode: &'static str,
    subject: String,
}
struct Provider {
    issuer: String,
    codes: Mutex<HashMap<String, Code>>,
    calls: AtomicUsize,
    rotated: AtomicBool,
    entered: Notify,
    release: Notify,
}
struct Fixture {
    provider: Arc<Provider>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
fn key_with_id(id: &str) -> CoreRsaPrivateSigningKey {
    CoreRsaPrivateSigningKey::from_pem(
        include_str!("fixtures/oidc-key.pem"),
        Some(JsonWebKeyId::new(id.into())),
    )
    .unwrap()
}
impl Fixture {
    async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = Arc::new(Provider {
            issuer: format!("http://{}", listener.local_addr().unwrap()),
            codes: Mutex::new(HashMap::new()),
            calls: AtomicUsize::new(0),
            rotated: AtomicBool::new(false),
            entered: Notify::new(),
            release: Notify::new(),
        });
        let router = axum::Router::new()
            .route("/.well-known/openid-configuration", get(metadata))
            .route(
                "/keys",
                get(|State(p): State<Arc<Provider>>| async move { Json(json!({"keys":[key_with_id(if p.rotated.load(Ordering::SeqCst) {"rotated"} else {"fixture"}).as_verification_key()]})) }),
            )
            .route("/token", post(exchange))
            .with_state(provider.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self { provider, server }
    }
    fn app(&self, pool: &PgPool) -> axum::Router {
        self.app_with_required(pool, false)
    }
    fn app_with_required(&self, pool: &PgPool, required: bool) -> axum::Router {
        let mut config = SecurityConfig::development();
        config.setup_token = Some(SETUP_TOKEN.into());
        config.authentication.oidc_required = required;
        config.authentication.provider = Some(convex_auth::auth_policy::ProviderConfig {
            issuer: self.provider.issuer.clone(),
            client_id: "convex-fixture".into(),
            client_secret: "fixture-secret".into(),
            display_name: "Fixture SSO".into(),
            enabled: true,
        });
        http::router_with_config(pool.clone(), config)
    }
    async fn flow(
        &self,
        app: &axum::Router,
        invitation: Option<&str>,
        mode: &'static str,
        subject: &str,
    ) -> (String, String) {
        self.flow_with_source(app, invitation, mode, subject, "")
            .await
    }
    async fn flow_with_source(
        &self,
        app: &axum::Router,
        invitation: Option<&str>,
        mode: &'static str,
        subject: &str,
        source: &str,
    ) -> (String, String) {
        let (status, body, cookie) = call(
            app,
            "POST",
            "/api/v1/auth/oidc/start",
            source,
            json!({"invitation_token":invitation}),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        let url = reqwest::Url::parse(body["authorization_url"].as_str().unwrap()).unwrap();
        let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params["code_challenge_method"], "S256");
        assert!(!params.contains_key("invitation_token"));
        let code = convex_auth::security::token();
        self.provider.codes.lock().await.insert(
            code.clone(),
            Code {
                nonce: params["nonce"].clone(),
                challenge: params["code_challenge"].clone(),
                mode,
                subject: subject.into(),
            },
        );
        (
            format!(
                "/api/v1/auth/oidc/callback?state={}&code={code}",
                params["state"]
            ),
            cookie,
        )
    }
}
async fn metadata(State(p): State<Arc<Provider>>) -> Json<Value> {
    Json(
        json!({"issuer":p.issuer,"authorization_endpoint":format!("{}/authorize",p.issuer),"token_endpoint":format!("{}/token",p.issuer),"jwks_uri":format!("{}/keys",p.issuer),"response_types_supported":["code"],"subject_types_supported":["public"],"id_token_signing_alg_values_supported":["RS256"]}),
    )
}
async fn exchange(
    State(p): State<Arc<Provider>>,
    Form(form): Form<HashMap<String, String>>,
) -> Json<Value> {
    p.calls.fetch_add(1, Ordering::SeqCst);
    let Some(code) = p.codes.lock().await.remove(form.get("code").unwrap()) else {
        return Json(json!({"error":"invalid_grant"}));
    };
    let verifier = PkceCodeVerifier::new(form.get("code_verifier").unwrap().clone());
    if PkceCodeChallenge::from_code_verifier_sha256(&verifier).as_str() != code.challenge
        || code.mode == "pkce"
    {
        return Json(json!({"error":"invalid_grant"}));
    }
    if code.mode == "blocked" {
        p.entered.notify_one();
        p.release.notified().await;
    }
    if code.mode == "missing" {
        return Json(json!({"access_token":"discard-me","token_type":"Bearer"}));
    }
    let issuer = if code.mode == "issuer" {
        "https://wrong.example"
    } else {
        &p.issuer
    };
    let audience = if code.mode == "audience" {
        "wrong-client"
    } else {
        "convex-fixture"
    };
    let nonce = if code.mode == "nonce" {
        "wrong-nonce"
    } else {
        &code.nonce
    };
    let expires = Utc::now() + Duration::seconds(if code.mode == "expired" { -300 } else { 300 });
    let mut claims = CoreIdTokenClaims::new(
        IssuerUrl::new(issuer.into()).unwrap(),
        vec![Audience::new(audience.into())],
        expires,
        Utc::now(),
        StandardClaims::new(SubjectIdentifier::new(code.subject))
            .set_email(Some(EndUserEmail::new("owner@example.com".into()))),
        EmptyAdditionalClaims {},
    )
    .set_nonce(Some(Nonce::new(nonce.into())));
    if code.mode == "azp" {
        claims = claims.set_authorized_party(Some(ClientId::new("wrong-client".into())));
    }
    if code.mode == "future" {
        claims = claims.set_issue_time(Utc::now() + Duration::hours(1));
    }
    if code.mode == "multi-aud" {
        claims = claims.set_audiences(vec![
            Audience::new("convex-fixture".into()),
            Audience::new("another".into()),
        ]);
    }
    claims = claims.set_auth_time(match code.mode {
        "no-auth" => None,
        "stale-auth" => Some(Utc::now() - Duration::minutes(10)),
        "future-auth" => Some(Utc::now() + Duration::hours(1)),
        _ => Some(Utc::now()),
    });
    if code.mode == "rotate" {
        p.rotated.store(true, Ordering::SeqCst);
    }
    if code.mode == "no-nonce" {
        claims = claims.set_nonce(None);
    }
    let signing_key = key_with_id(if p.rotated.load(Ordering::SeqCst) {
        "rotated"
    } else {
        "fixture"
    });
    let token = CoreIdToken::new(
        claims,
        &signing_key,
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        None,
        None,
    )
    .unwrap();
    let mut encoded = serde_json::to_value(token)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned();
    if code.mode == "signature" {
        let position = encoded.rfind('.').unwrap() + 1;
        let replacement = if &encoded[position..position + 1] == "A" {
            "B"
        } else {
            "A"
        };
        encoded.replace_range(position..position + 1, replacement);
    }
    Json(
        json!({"access_token":"discard-me","refresh_token":"also-discard-me","token_type":"Bearer","id_token":encoded}),
    )
}
async fn callback(app: &axum::Router, path: &str, cookie: &str) -> (bool, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let location = response.headers()["location"].to_str().unwrap();
    assert!(
        location == format!("{ORIGIN}/auth/oidc?outcome=success")
            || location == format!("{ORIGIN}/auth/oidc?outcome=failed")
    );
    let success = location.ends_with("=success");
    let cookie = response
        .headers()
        .get("set-cookie")
        .map(|h| h.to_str().unwrap().split(';').next().unwrap().into())
        .unwrap_or_default();
    (success, cookie)
}
async fn invite(app: &axum::Router, owner: &str, space: &str) -> String {
    let (status, body, _) = call(
        app,
        "POST",
        &format!("/api/v1/workspaces/{space}/invitations"),
        owner,
        json!({"role":"member"}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    body["url"]
        .as_str()
        .unwrap()
        .split('#')
        .nth(1)
        .unwrap()
        .into()
}
#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn signed_oidc_callbacks_preserve_identity_and_invitation_atomicity(pool: PgPool) {
    let fixture = Fixture::new().await;
    let app = fixture.app(&pool);
    let (owner, space, local_user) = initialized(&app).await;
    let token = invite(&app, &owner, &space).await;
    for mode in [
        "signature",
        "issuer",
        "audience",
        "nonce",
        "expired",
        "azp",
        "future",
        "multi-aud",
        "no-nonce",
        "missing",
        "pkce",
    ] {
        let (path, cookie) = fixture
            .flow(&app, Some(&token), mode, "external-member")
            .await;
        assert!(!callback(&app, &path, &cookie).await.0, "{mode}");
        let count = fixture.provider.calls.load(Ordering::SeqCst);
        assert!(!callback(&app, &path, &cookie).await.0);
        assert_eq!(
            fixture.provider.calls.load(Ordering::SeqCst),
            count,
            "replay contacted provider"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.users")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert!(
            !sqlx::query_scalar::<_, bool>(
                "SELECT accepted_at IS NOT NULL FROM convex.invitations WHERE token_hash=$1"
            )
            .bind(convex_auth::security::digest(&token))
            .fetch_one(&pool)
            .await
            .unwrap()
        );
    }
    let (path, cookie) = fixture.flow(&app, None, "valid", "external-member").await;
    assert!(
        !callback(&app, &path, &cookie).await.0,
        "public provisioning"
    );
    let (path, cookie) = fixture
        .flow(&app, Some(&token), "rotate", "external-member")
        .await;
    let count = fixture.provider.calls.load(Ordering::SeqCst);
    assert!(
        !callback(&app, &path, "").await.0,
        "missing browser binding"
    );
    assert_eq!(fixture.provider.calls.load(Ordering::SeqCst), count);
    let (first, second) = tokio::join!(
        callback(&app, &path, &cookie),
        callback(&app, &path, &cookie)
    );
    assert_ne!(first.0, second.0, "exactly one callback must succeed");
    assert_eq!(fixture.provider.calls.load(Ordering::SeqCst), count + 1);
    let session = if first.0 { first.1 } else { second.1 };
    let (status, account, _) = call(&app, "GET", "/api/v1/auth/me", &session, Value::Null).await;
    assert_eq!(status, 200);
    assert_ne!(account["user"]["id"], local_user);
    assert_eq!(account["workspaces"][0]["id"], space);
    let oidc_user = account["user"]["id"].clone();
    let (path, cookie) = fixture.flow(&app, None, "valid", "external-member").await;
    let (success, session) = callback(&app, &path, &cookie).await;
    assert!(success);
    let (_, account, _) = call(&app, "GET", "/api/v1/auth/me", &session, Value::Null).await;
    assert_eq!(account["user"]["id"], oidc_user);
    // Protected requests still succeed after the provider stops.
    fixture.server.abort();
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &session, Value::Null)
            .await
            .0,
        200
    );
}
#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn callback_rechecks_invitation_after_exchange_without_holding_database_gate(pool: PgPool) {
    let fixture = Fixture::new().await;
    let app = fixture.app(&pool);
    let (owner, space, _) = initialized(&app).await;
    for change in [
        "revoked",
        "inviter-demoted",
        "inviter-disabled",
        "local-accept",
    ] {
        let token = invite(&app, &owner, &space).await;
        let (path, cookie) = fixture
            .flow(&app, Some(&token), "blocked", "new-member")
            .await;
        let callback_app = app.clone();
        let pending = tokio::spawn(async move { callback(&callback_app, &path, &cookie).await });
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            fixture.provider.entered.notified(),
        )
        .await
        .unwrap();
        if change == "local-accept" {
            let (status, _, _) = tokio::time::timeout(std::time::Duration::from_secs(2), call(&app, "POST", "/api/v1/invitations/accept", "", json!({"token":token,"email":"local-winner@example.com","password":PASSWORD,"display_name":"Local winner"}))).await.unwrap();
            assert_eq!(
                status, 200,
                "local acceptance must finish while exchange is blocked"
            );
        } else {
            // The installation gate must remain free during provider work.
            let mut tx = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                convex_auth::postgres::accounts::transaction(&pool),
            )
            .await
            .unwrap()
            .unwrap();
            let statement = match change {
                "revoked" => "UPDATE convex.invitations SET revoked_at=now() WHERE token_hash=$1",
                "inviter-demoted" => {
                    "UPDATE convex.workspace_memberships SET role='member' WHERE id=(SELECT inviter_membership_id FROM convex.invitations WHERE token_hash=$1)"
                }
                _ => {
                    "UPDATE convex.users SET disabled_at=now() WHERE id=(SELECT m.user_id FROM convex.invitations i JOIN convex.workspace_memberships m ON m.id=i.inviter_membership_id WHERE i.token_hash=$1)"
                }
            };
            sqlx::query(statement)
                .bind(convex_auth::security::digest(&token))
                .execute(&mut *tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
        fixture.provider.release.notify_one();
        assert!(!pending.await.unwrap().0, "{change}");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.users")
                .fetch_one(&pool)
                .await
                .unwrap(),
            if change == "local-accept" { 2 } else { 1 }
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.oidc_identities")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0,
            "failed callback leaves no identity"
        );
        sqlx::query("UPDATE convex.workspace_memberships SET role='owner' WHERE role='member' AND user_id=(SELECT id FROM convex.users WHERE is_instance_admin)").execute(&pool).await.unwrap();
        sqlx::query("UPDATE convex.users SET disabled_at=NULL")
            .execute(&pool)
            .await
            .unwrap();
    }
    // Also verify the reverse ordering: OIDC wins and local acceptance cannot reuse it.
    let token = invite(&app, &owner, &space).await;
    let (path, cookie) = fixture
        .flow(&app, Some(&token), "valid", "oidc-winner")
        .await;
    let (success, member) = callback(&app, &path, &cookie).await;
    assert!(success);
    assert_eq!(call(&app, "POST", "/api/v1/invitations/accept", "", json!({"token":token,"email":"local-loser@example.com","password":PASSWORD,"display_name":"Local loser"})).await.0, 400);
    let (_, account, _) = call(&app, "GET", "/api/v1/auth/me", &member, Value::Null).await;
    let id = uuid::Uuid::parse_str(account["user"]["id"].as_str().unwrap()).unwrap();
    sqlx::query("UPDATE convex.workspace_memberships SET removed_at=now() WHERE user_id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let (path, cookie) = fixture.flow(&app, None, "valid", "oidc-winner").await;
    let (success, member) = callback(&app, &path, &cookie).await;
    assert!(success);
    let (_, account, _) = call(&app, "GET", "/api/v1/auth/me", &member, Value::Null).await;
    assert!(
        account["workspaces"].as_array().unwrap().is_empty(),
        "repeat provider login must not restore removed membership"
    );
}

fn state_hash(path: &str) -> String {
    let url = reqwest::Url::parse(&format!("{ORIGIN}{path}")).unwrap();
    convex_auth::security::digest(&url.query_pairs().find(|(key, _)| key == "state").unwrap().1)
}
#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn oidc_flow_binding_expiry_intent_denial_and_logout_are_checked_before_provider_calls(
    pool: PgPool,
) {
    let fixture = Fixture::new().await;
    let app = fixture.app(&pool);
    let (owner, _, _) = initialized(&app).await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/oidc/start")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    for alteration in ["expiry", "origin", "intent", "browser", "denial"] {
        let (path, cookie) = fixture
            .flow_with_source(&app, None, "valid", "external-member", &owner)
            .await;
        let hash = state_hash(&path);
        match alteration {
            "expiry" => {
                sqlx::query("UPDATE convex.oidc_flows SET expires_at=now()-interval '1 second' WHERE state_hash=$1").bind(&hash).execute(&pool).await.unwrap();
            }
            "origin" => {
                sqlx::query("UPDATE convex.oidc_flows SET public_origin='https://other.example' WHERE state_hash=$1").bind(&hash).execute(&pool).await.unwrap();
            }
            "intent" => {
                sqlx::query("UPDATE convex.oidc_flows SET intent='link' WHERE state_hash=$1")
                    .bind(&hash)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            _ => {}
        }
        let bound_cookie = if alteration == "browser" {
            format!("convex_oidc_browser={}", "a".repeat(64))
        } else {
            format!("{cookie}; {owner}")
        };
        let path = if alteration == "denial" {
            format!(
                "{}&error=access_denied",
                path.split("&code=").next().unwrap()
            )
        } else {
            path
        };
        assert!(!callback(&app, &path, &bound_cookie).await.0);
        assert_eq!(fixture.provider.calls.load(Ordering::SeqCst), 0);
        if alteration == "denial" {
            let consumed: bool = sqlx::query_scalar("SELECT consumed_at IS NOT NULL AND nonce IS NULL AND pkce_verifier IS NULL FROM convex.oidc_flows WHERE state_hash=$1").bind(&hash).fetch_one(&pool).await.unwrap();
            assert!(consumed);
        }
    }
    let (path, cookie) = fixture
        .flow_with_source(&app, None, "valid", "external-member", &owner)
        .await;
    assert_eq!(
        call(&app, "POST", "/api/v1/auth/logout", &owner, Value::Null)
            .await
            .0,
        200
    );
    assert!(!callback(&app, &path, &format!("{cookie}; {owner}")).await.0);
    assert_eq!(fixture.provider.calls.load(Ordering::SeqCst), 0);
}

impl Fixture {
    async fn method_flow(
        &self,
        app: &axum::Router,
        path: &str,
        source: &str,
        body: Value,
        mode: &'static str,
        subject: &str,
    ) -> (String, String) {
        let (status, body, cookie) = call(app, "POST", path, source, body).await;
        assert_eq!(status, 200, "{body}");
        let url = reqwest::Url::parse(body["authorization_url"].as_str().unwrap()).unwrap();
        let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params["max_age"], "0");
        assert_eq!(params["prompt"], "login");
        let code = convex_auth::security::token();
        self.provider.codes.lock().await.insert(
            code.clone(),
            Code {
                nonce: params["nonce"].clone(),
                challenge: params["code_challenge"].clone(),
                mode,
                subject: subject.into(),
            },
        );
        (
            format!(
                "/api/v1/auth/oidc/callback?state={}&code={code}",
                params["state"]
            ),
            {
                let mut parts: HashMap<_, _> = source
                    .split(';')
                    .filter_map(|p| p.trim().split_once('='))
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect();
                let (k, v) = cookie.split_once('=').unwrap();
                parts.insert(k.into(), v.into());
                parts
                    .into_iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            },
        )
    }
}
async fn method_callback(app: &axum::Router, path: &str, cookie: &str) -> (String, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response.headers()["location"].to_str().unwrap().to_owned();
    let mut cookies: HashMap<String, String> = cookie
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
    for header in response.headers().get_all("set-cookie") {
        let (key, value) = header
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .split_once('=')
            .unwrap();
        if value.is_empty() {
            cookies.remove(key);
        } else {
            cookies.insert(key.into(), value.into());
        }
    }
    (
        location,
        cookies
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; "),
    )
}
async fn local_member(
    app: &axum::Router,
    owner: &str,
    space: &str,
    email: &str,
) -> (String, String) {
    let invitation = invite(app, owner, space).await;
    let (status, _, cookie) = call(
        app,
        "POST",
        "/api/v1/invitations/accept",
        "",
        json!({"token":invitation,"email":email,"password":PASSWORD,"display_name":"Local member"}),
    )
    .await;
    assert_eq!(status, 200);
    let (_, account, _) = call(app, "GET", "/api/v1/auth/me", &cookie, Value::Null).await;
    (cookie, account["user"]["id"].as_str().unwrap().into())
}
#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn local_oidc_linking_requires_fresh_proofs_preserves_identity_and_rejects_conflicts(
    pool: PgPool,
) {
    let fixture = Fixture::new().await;
    let app = fixture.app(&pool);
    let (owner, space, user) = initialized(&app).await;
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/oidc/link",
            &owner,
            json!({"current_password":"wrong password"})
        )
        .await
        .0,
        401
    );
    for mode in ["no-auth", "stale-auth", "future-auth"] {
        let (path, cookie) = fixture
            .method_flow(
                &app,
                "/api/v1/auth/methods/oidc/link",
                &owner,
                json!({"current_password":PASSWORD}),
                mode,
                "owner-identity",
            )
            .await;
        assert!(
            method_callback(&app, &path, &cookie)
                .await
                .0
                .ends_with("=failed")
        );
    }
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/link",
            &owner,
            json!({"current_password":PASSWORD}),
            "valid",
            "owner-identity",
        )
        .await;
    let (location, linked) = method_callback(&app, &path, &cookie).await;
    assert!(location.ends_with("/account?method=linked"));
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        401
    );
    let (_, account, _) = call(&app, "GET", "/api/v1/auth/me", &linked, Value::Null).await;
    assert_eq!(account["user"]["id"], user);
    assert_eq!(account["workspaces"][0]["id"], space);
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/oidc/prove",
            &linked,
            json!({"action":"remove_local"})
        )
        .await
        .0,
        409,
        "last local administrator"
    );
    let (_, methods, _) = call(&app, "GET", "/api/v1/auth/methods", &linked, Value::Null).await;
    let identity = methods["oidc"][0]["id"].clone();
    let (status, _, local) = call(
        &app,
        "POST",
        "/api/v1/auth/methods/oidc/remove",
        &linked,
        json!({"identity_id":identity,"current_password":PASSWORD}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &linked, Value::Null)
            .await
            .0,
        401
    );
    // An unlinked subject remains reserved to its original account and can be reattached there.
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/link",
            &local,
            json!({"current_password":PASSWORD}),
            "valid",
            "owner-identity",
        )
        .await;
    let (_, linked) = method_callback(&app, &path, &cookie).await;
    let (other, other_user) = local_member(&app, &linked, &space, "other@example.com").await;
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/link",
            &other,
            json!({"current_password":PASSWORD}),
            "valid",
            "owner-identity",
        )
        .await;
    assert!(
        method_callback(&app, &path, &cookie)
            .await
            .0
            .ends_with("=failed")
    );
    let (_, account, _) = call(&app, "GET", "/api/v1/auth/me", &other, Value::Null).await;
    assert_eq!(account["user"]["id"], other_user);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.oidc_identities")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}
#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn oidc_local_attachment_and_removal_require_exact_single_use_session_bound_proofs(
    pool: PgPool,
) {
    let fixture = Fixture::new().await;
    let app = fixture.app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let invitation = invite(&app, &owner, &space).await;
    let (path, cookie) = fixture
        .flow(&app, Some(&invitation), "valid", "oidc-member")
        .await;
    let (_, member) = method_callback(&app, &path, &cookie).await;
    let (_, original, _) = call(&app, "GET", "/api/v1/auth/me", &member, Value::Null).await;
    let request = json!({"email":"member@example.com","password":PASSWORD});
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/add",
            &member,
            request.clone()
        )
        .await
        .0,
        401
    );
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/prove",
            &member,
            json!({"action":"add_local"}),
            "valid",
            "wrong-subject",
        )
        .await;
    assert!(
        method_callback(&app, &path, &cookie)
            .await
            .0
            .ends_with("=failed")
    );
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/prove",
            &member,
            json!({"action":"add_local"}),
            "valid",
            "oidc-member",
        )
        .await;
    let (location, proof) = method_callback(&app, &path, &cookie).await;
    assert!(location.ends_with("/account?method=proof"));
    let proof_part = proof
        .split(';')
        .find(|p| p.trim().starts_with("convex_method_proof="))
        .unwrap();
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/add",
            &format!("{owner}; {proof_part}"),
            request.clone()
        )
        .await
        .0,
        401
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/remove",
            &proof,
            Value::Null
        )
        .await
        .0,
        401,
        "proof operation mismatch"
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/add",
            &proof,
            json!({"email":"owner@example.com","password":PASSWORD})
        )
        .await
        .0,
        409
    );
    let (status, _, local) = call(
        &app,
        "POST",
        "/api/v1/auth/methods/local/add",
        &proof,
        request,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &member, Value::Null)
            .await
            .0,
        401
    );
    let (_, account, _) = call(&app, "GET", "/api/v1/auth/me", &local, Value::Null).await;
    assert_eq!(account["user"]["id"], original["user"]["id"]);
    assert_eq!(account["workspaces"], original["workspaces"]);
    let recovery = convex_auth::security::token();
    sqlx::query("INSERT INTO convex.recovery_tokens (id,user_id,token_hash) VALUES ($1,$2,$3)")
        .bind(uuid::Uuid::new_v4())
        .bind(uuid::Uuid::parse_str(account["user"]["id"].as_str().unwrap()).unwrap())
        .bind(convex_auth::security::digest(&recovery))
        .execute(&pool)
        .await
        .unwrap();
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/prove",
            &local,
            json!({"action":"remove_local"}),
            "valid",
            "oidc-member",
        )
        .await;
    let (_, proof) = method_callback(&app, &path, &cookie).await;
    let (status, _, remaining) = call(
        &app,
        "POST",
        "/api/v1/auth/methods/local/remove",
        &proof,
        Value::Null,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.recovery_tokens")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/reset",
            "",
            json!({"token":recovery,"password":PASSWORD})
        )
        .await
        .0,
        400,
        "removed password cannot be recreated with its old recovery token"
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/login",
            "",
            json!({"email":"member@example.com","password":PASSWORD})
        )
        .await
        .0,
        401
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &remaining, Value::Null)
            .await
            .0,
        200
    );
    let (_, methods, _) = call(&app, "GET", "/api/v1/auth/methods", &remaining, Value::Null).await;
    assert!(methods["local_email"].is_null());
    assert!(!methods["local_usable"].as_bool().unwrap());
}
#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn concurrent_method_removal_keeps_a_usable_method_and_revoked_link_flows_cannot_mutate(
    pool: PgPool,
) {
    let fixture = Fixture::new().await;
    let app = fixture.app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let (member, _) = local_member(&app, &owner, &space, "race@example.com").await;
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/link",
            &member,
            json!({"current_password":PASSWORD}),
            "blocked",
            "race-member",
        )
        .await;
    let pending_app = app.clone();
    let pending = tokio::spawn(async move { method_callback(&pending_app, &path, &cookie).await });
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        fixture.provider.entered.notified(),
    )
    .await
    .unwrap();
    assert_eq!(
        call(&app, "POST", "/api/v1/auth/logout", &member, Value::Null)
            .await
            .0,
        200
    );
    fixture.provider.release.notify_one();
    assert!(pending.await.unwrap().0.ends_with("=failed"));
    let (_, _, member) = call(
        &app,
        "POST",
        "/api/v1/auth/login",
        "",
        json!({"email":"race@example.com","password":PASSWORD}),
    )
    .await;
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/link",
            &member,
            json!({"current_password":PASSWORD}),
            "valid",
            "race-member",
        )
        .await;
    let (_, linked) = method_callback(&app, &path, &cookie).await;
    let (_, methods, _) = call(&app, "GET", "/api/v1/auth/methods", &linked, Value::Null).await;
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/prove",
            &linked,
            json!({"action":"remove_local"}),
            "valid",
            "race-member",
        )
        .await;
    let (location, proof) = method_callback(&app, &path, &cookie).await;
    assert!(location.ends_with("/account?method=proof"));
    let (a, b) = tokio::join!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/remove",
            &proof,
            Value::Null
        ),
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/oidc/remove",
            &proof,
            json!({"identity_id":methods["oidc"][0]["id"],"current_password":PASSWORD})
        )
    );
    assert!(
        matches!((a.0, b.0), (200, 401) | (401, 200)),
        "{} {}",
        a.0,
        b.0
    );
    let remaining = if a.0 == 200 { a.2 } else { b.2 };
    let (_, methods, _) = call(&app, "GET", "/api/v1/auth/methods", &remaining, Value::Null).await;
    assert!(
        methods["local_usable"].as_bool().unwrap()
            || methods["oidc"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["usable"] == true)
    );
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn method_proofs_expire_are_session_bound_and_respect_required_oidc(pool: PgPool) {
    let fixture = Fixture::new().await;
    let app = fixture.app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let invitation = invite(&app, &owner, &space).await;
    let (path, cookie) = fixture
        .flow(&app, Some(&invitation), "valid", "proof-member")
        .await;
    let (_, member) = method_callback(&app, &path, &cookie).await;
    let (path, cookie) = fixture
        .method_flow(
            &app,
            "/api/v1/auth/methods/oidc/prove",
            &member,
            json!({"action":"add_local"}),
            "valid",
            "proof-member",
        )
        .await;
    let (location, proof) = method_callback(&app, &path, &cookie).await;
    assert!(location.ends_with("/account?method=proof"));
    let bounded: bool = sqlx::query_scalar(
        "SELECT expires_at <= verified_at + interval '5 minutes' FROM convex.method_proofs",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(bounded);
    // A separate login to the same account must not inherit the first session's proof.
    let (path, cookie) = fixture.flow(&app, None, "valid", "proof-member").await;
    let (_, other) = method_callback(&app, &path, &cookie).await;
    let session = other
        .split(';')
        .find(|p| p.trim().starts_with("convex_session="))
        .unwrap();
    let crossed = proof
        .split(';')
        .filter(|p| !p.trim().starts_with("convex_session="))
        .chain(std::iter::once(session))
        .collect::<Vec<_>>()
        .join(";");
    let request = json!({"email":"proof-member@example.com","password":PASSWORD});
    let wrong_browser = proof
        .split(';')
        .map(|part| {
            if part.trim().starts_with("convex_oidc_browser=") {
                format!("convex_oidc_browser={}", "0".repeat(64))
            } else {
                part.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(";");
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/add",
            &wrong_browser,
            request.clone()
        )
        .await
        .0,
        401,
        "proof cannot cross browser binding"
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/add",
            &crossed,
            request.clone()
        )
        .await
        .0,
        401
    );
    sqlx::query("UPDATE convex.method_proofs SET expires_at=now()-interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/add",
            &proof,
            request.clone()
        )
        .await
        .0,
        401
    );
    let (_, methods, _) = call(&app, "GET", "/api/v1/auth/methods", &proof, Value::Null).await;
    assert!(methods["pending_action"].is_null());
    // End the replica lease before enabling installation-wide enforcement.
    drop(app);
    wait_for_policy_leases(&pool).await;
    let app = fixture.app_with_required(&pool, true);
    let (_, methods, _) = call(&app, "GET", "/api/v1/auth/methods", &member, Value::Null).await;
    assert_eq!(methods["can_add_local"], false);
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/oidc/prove",
            &member,
            json!({"action":"add_local"})
        )
        .await
        .0,
        403
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/methods/local/add",
            &member,
            request
        )
        .await
        .0,
        403
    );
}
