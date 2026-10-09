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
        let mut config = SecurityConfig::development();
        config.setup_token = Some(SETUP_TOKEN.into());
        config.authentication.provider = Some(convex::auth_policy::ProviderConfig {
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
        let code = convex::security::token();
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
            .bind(convex::security::digest(&token))
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
    // The installation gate must be free while waiting on the provider.
    let mut tx = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        convex::accounts::transaction(&pool),
    )
    .await
    .unwrap()
    .unwrap();
    sqlx::query("UPDATE convex.invitations SET revoked_at=now() WHERE token_hash=$1")
        .bind(convex::security::digest(&token))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    fixture.provider.release.notify_one();
    assert!(!pending.await.unwrap().0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.users")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

fn state_hash(path: &str) -> String {
    let url = reqwest::Url::parse(&format!("{ORIGIN}{path}")).unwrap();
    convex::security::digest(&url.query_pairs().find(|(key, _)| key == "state").unwrap().1)
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
