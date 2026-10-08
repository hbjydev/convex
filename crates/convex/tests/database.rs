use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use convex::{MIGRATOR, http};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn empty_database_migrates_and_responds_with_documented_health(
    pool_options: PgPoolOptions,
    connect_options: PgConnectOptions,
) {
    let pool = pool_options
        .connect_with(connect_options.options([("search_path", "public")]))
        .await
        .unwrap();
    // Reapplying migrations must be harmless and preserve the migration ledger.
    MIGRATOR.run(&pool).await.unwrap();
    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(applied, 3);

    let response = http::router(pool)
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let contract = serde_json::to_value(http::openapi()).unwrap();
    let schema = json!({ "$ref": "#/components/schemas/HealthResponse", "components": contract["components"] });
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&body)
        .unwrap();
    assert_eq!(body["status"], "ready");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[sqlx::test(migrations = false)]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn reachable_but_unmigrated_database_is_not_ready(pool: PgPool) {
    let response = http::router(pool)
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

use convex::security::SecurityConfig;

const ORIGIN: &str = "http://127.0.0.1:5173";
const SETUP_TOKEN: &str = "an-installer-authorized-setup-token-for-testing";
const PASSWORD: &str = "a very strong test password";

fn app(pool: &PgPool) -> axum::Router {
    let mut config = SecurityConfig::development();
    config.setup_token = Some(SETUP_TOKEN.into());
    http::router_with_config(pool.clone(), config)
}
fn setup_request() -> Value {
    json!({"setup_token":SETUP_TOKEN,"email":"Owner@Example.com","password":PASSWORD,"display_name":"Owner","workspace_name":"Acme","workspace_slug":"acme","team_name":"Engineering","team_key":"eng"})
}

async fn call(
    app: &axum::Router,
    method: &str,
    path: &str,
    cookie: &str,
    data: Value,
) -> (u16, Value, String) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("origin", ORIGIN)
        .header("x-convex-csrf", "1")
        .header("content-type", "application/json")
        .header("cookie", cookie)
        .body(Body::from(serde_json::to_vec(&data).unwrap()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap().to_owned())
        .unwrap_or_default();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    // Every exercised response must be in the generated contract, including errors.
    let contract = serde_json::to_value(http::openapi()).unwrap();
    let matched = contract["paths"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(pattern, _)| {
            let a: Vec<_> = pattern.split('/').collect();
            let b: Vec<_> = path.split('/').collect();
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.starts_with('{') || *a == b)
        })
        .unwrap()
        .1;
    let schema = &matched[method.to_lowercase()]["responses"][status.to_string()]["content"]["application/json"]
        ["schema"];
    assert!(
        !schema.is_null(),
        "Undocumented response {method} {path} {status}: {body}"
    );
    let schema = json!({"allOf":[schema],"components":contract["components"]});
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&body)
        .unwrap();
    (status, body, cookie)
}
async fn initialized(app: &axum::Router) -> (String, String, String) {
    let (status, user, cookie) = call(app, "POST", "/api/v1/auth/setup", "", setup_request()).await;
    assert_eq!(status, 200, "{user}");
    let (_, account, _) = call(app, "GET", "/api/v1/auth/me", &cookie, Value::Null).await;
    assert_eq!(account["workspaces"][0]["slug"], "acme");
    (
        cookie,
        account["workspaces"][0]["id"].as_str().unwrap().into(),
        user["id"].as_str().unwrap().into(),
    )
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn workspace_slugs_are_validated_unique_and_stable(pool: PgPool) {
    let app = app(&pool);
    let mut invalid_setup = setup_request();
    invalid_setup["workspace_slug"] = json!("invalid/slug");
    assert_eq!(
        call(&app, "POST", "/api/v1/auth/setup", "", invalid_setup)
            .await
            .0,
        400
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM convex.users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let (owner, space, _) = initialized(&app).await;
    let request = json!({"name":"Other","slug":" ACME ","team_name":"Other","team_key":"OPS"});
    let (status, error, _) = call(&app, "POST", "/api/v1/workspaces", &owner, request).await;
    assert_eq!(status, 409);
    assert!(error["message"].as_str().unwrap().contains("workspace URL"));
    let request = json!({"name":"Other","slug":"other-team","team_name":"Other","team_key":"OPS"});
    let (a, b) = tokio::join!(
        call(&app, "POST", "/api/v1/workspaces", &owner, request.clone()),
        call(&app, "POST", "/api/v1/workspaces", &owner, request),
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/rename"),
            &owner,
            json!({"name":"Renamed"})
        )
        .await
        .0,
        200
    );
    let (_, detail, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(detail["workspace"]["name"], "Renamed");
    assert_eq!(detail["workspace"]["slug"], "acme");
    let token = invitation(&app, &owner, &space, "member").await;
    let (status, workspace, _) = join(&app, &token, "slug-member@example.com").await;
    assert_eq!(status, 200);
    assert_eq!(workspace["slug"], "acme");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM convex.workspaces")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = false)]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn workspace_slug_migration_preserves_existing_workspaces(pool: PgPool) {
    sqlx::raw_sql(include_str!(
        "../../../migrations/202610080001_application_schema.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/202610080002_accounts_workspaces.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for name in ["Acme", "Acme", "你好", &"A".repeat(100)] {
        sqlx::query("INSERT INTO convex.workspaces (id,name) VALUES ($1,$2)")
            .bind(uuid::Uuid::new_v4())
            .bind(name)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::raw_sql(include_str!(
        "../../../migrations/202610080003_workspace_slugs.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let rows: Vec<(uuid::Uuid, String, String)> =
        sqlx::query_as("SELECT id,name,slug FROM convex.workspaces")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 4);
    for (id, _, slug) in &rows {
        assert_eq!(convex::security::workspace_slug(slug).unwrap(), *slug);
        assert!(slug.ends_with(&id.to_string()));
    }
    let unique: std::collections::HashSet<_> = rows.iter().map(|row| &row.2).collect();
    assert_eq!(unique.len(), 4);
}
async fn invitation(app: &axum::Router, cookie: &str, space: &str, role: &str) -> String {
    let (status, body, _) = call(
        app,
        "POST",
        &format!("/api/v1/workspaces/{space}/invitations"),
        cookie,
        json!({"role":role}),
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
async fn join(app: &axum::Router, token: &str, email: &str) -> (u16, Value, String) {
    call(
        app,
        "POST",
        "/api/v1/invitations/accept",
        "",
        json!({"token":token,"email":email,"password":PASSWORD,"display_name":email}),
    )
    .await
}
async fn login(app: &axum::Router, email: &str, password: &str) -> (u16, Value, String) {
    call(
        app,
        "POST",
        "/api/v1/auth/login",
        "",
        json!({"email":email,"password":password}),
    )
    .await
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn installer_authority_and_atomic_setup(pool: PgPool) {
    let app = app(&pool);
    let mut wrong = setup_request();
    wrong["setup_token"] = json!("wrong");
    assert_eq!(
        call(&app, "POST", "/api/v1/auth/setup", "", wrong).await.0,
        403
    );
    let (a, b) = tokio::join!(
        call(&app, "POST", "/api/v1/auth/setup", "", setup_request()),
        call(&app, "POST", "/api/v1/auth/setup", "", setup_request())
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    let cookie = if a.0 == 200 { a.2 } else { b.2 };
    assert!(cookie.starts_with("convex_session="));
    assert_eq!(login(&app, "owner@example.com", PASSWORD).await.0, 200);
    let (_, bootstrap, _) = call(&app, "GET", "/api/v1/auth/bootstrap", "", Value::Null).await;
    assert_eq!(bootstrap["setup_required"], false);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.users")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.workflow_statuses")
            .fetch_one(&pool)
            .await
            .unwrap(),
        5
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/logout")
        .header("cookie", &cookie)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/logout")
        .header("origin", "https://attacker.example")
        .header("x-convex-csrf", "1")
        .header("cookie", &cookie)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("origin", ORIGIN)
        .header("x-convex-csrf", "1")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers()["content-type"], "application/json");
    let unconfigured = http::router(pool.clone());
    assert_eq!(
        call(
            &unconfigured,
            "POST",
            "/api/v1/auth/setup",
            "",
            setup_request()
        )
        .await
        .0,
        403
    );
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn invitation_single_use_roles_and_membership_removal(pool: PgPool) {
    let app = app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let token = invitation(&app, &owner, &space, "member").await;
    let (a, b) = tokio::join!(
        join(&app, &token, "one@example.com"),
        join(&app, &token, "two@example.com")
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [200, 400]);
    let (email, member) = if a.0 == 200 {
        ("one@example.com", a.2)
    } else {
        ("two@example.com", b.2)
    };
    let (_, detail, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &member,
        Value::Null,
    )
    .await;
    assert_eq!(detail["teams"][0]["key"], "ENG");
    assert_eq!(detail["teams"][0]["statuses"][1]["category"], "unstarted");
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/teams"),
            &member,
            json!({"name":"Forbidden","key":"NO"})
        )
        .await
        .0,
        403
    );
    let target = detail["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "member")
        .unwrap();
    let member_id = target["id"].clone();
    let owner_id = detail["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "owner")
        .unwrap()["id"]
        .clone();
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/members"),
            &owner,
            json!({"membership_id":owner_id,"remove":true})
        )
        .await
        .0,
        409
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/members"),
            &owner,
            json!({"membership_id":member_id,"remove":true})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &member, Value::Null)
            .await
            .0,
        401
    );
    let (_, account, fresh) = login(&app, email, PASSWORD).await;
    assert!(account["id"].is_string());
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/v1/workspaces/{space}"),
            &fresh,
            Value::Null
        )
        .await
        .0,
        404
    );
    let restore = invitation(&app, &owner, &space, "member").await;
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/invitations/accept",
            &fresh,
            json!({"token":restore})
        )
        .await
        .0,
        200
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM convex.workspace_memberships WHERE id=$1")
            .bind(member_id.as_str().unwrap().parse::<uuid::Uuid>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn workspace_isolation_admin_limits_and_inviter_authority(pool: PgPool) {
    let app = app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let admin_token = invitation(&app, &owner, &space, "admin").await;
    let (_, _, admin) = join(&app, &admin_token, "admin@example.com").await;
    let (_, other, _) = call(
        &app,
        "POST",
        "/api/v1/workspaces",
        &owner,
        json!({"name":"Private","slug":"private","team_name":"Other","team_key":"OPS"}),
    )
    .await;
    let other = other["id"].as_str().unwrap();
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/v1/workspaces/{other}"),
            &admin,
            Value::Null
        )
        .await
        .0,
        404
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{other}/teams"),
            &admin,
            json!({"name":"Guess","key":"NO"})
        )
        .await
        .0,
        404
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/invitations"),
            &admin,
            json!({"role":"admin"})
        )
        .await
        .0,
        403
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/workspaces",
            &admin,
            json!({"name":"Guess","slug":"guess","team_name":"Guess","team_key":"NO"})
        )
        .await
        .0,
        403
    );
    let pending = invitation(&app, &admin, &space, "member").await;
    let (_, detail, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    let admin_id = &detail["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "admin")
        .unwrap()["id"];
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/members"),
            &owner,
            json!({"membership_id":admin_id,"role":"member","remove":false})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &admin, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(join(&app, &pending, "third@example.com").await.0, 400);
    let revoked = invitation(&app, &owner, &space, "member").await;
    sqlx::query("UPDATE convex.invitations SET revoked_at=now() WHERE token_hash=$1")
        .bind(convex::security::digest(&revoked))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(join(&app, &revoked, "third@example.com").await.0, 400);
    let expired = invitation(&app, &owner, &space, "member").await;
    sqlx::query(
        "UPDATE convex.invitations SET expires_at=now()-interval '1 second' WHERE token_hash=$1",
    )
    .bind(convex::security::digest(&expired))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(join(&app, &expired, "third@example.com").await.0, 400);
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn passwords_sessions_disable_and_operator_recovery(pool: PgPool) {
    let app = app(&pool);
    let (owner, space, owner_id) = initialized(&app).await;
    let token = invitation(&app, &owner, &space, "member").await;
    let (_, _, member) = join(&app, &token, "member@example.com").await;
    let (_, _, other) = login(&app, "member@example.com", PASSWORD).await;
    let new_password = "a new strong password";
    let (status, _, fresh) = call(
        &app,
        "POST",
        "/api/v1/auth/password",
        &member,
        json!({"current_password":PASSWORD,"new_password":new_password}),
    )
    .await;
    assert_eq!(status, 200);
    assert_ne!(fresh, member);
    for old in [&member, &other] {
        assert_eq!(
            call(&app, "GET", "/api/v1/auth/me", old, Value::Null)
                .await
                .0,
            401
        );
    }
    assert_eq!(login(&app, "member@example.com", PASSWORD).await.0, 401);
    let (_, detail, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    let target = detail["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "member")
        .unwrap()["user_id"]
        .as_str()
        .unwrap();
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/admin/accounts/disable",
            &fresh,
            json!({"user_id":owner_id,"disabled":true})
        )
        .await
        .0,
        403
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/admin/accounts/disable",
            &owner,
            json!({"user_id":owner_id,"disabled":true})
        )
        .await
        .0,
        409
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/admin/accounts/disable",
            &owner,
            json!({"user_id":target,"disabled":true})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &fresh, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(login(&app, "member@example.com", new_password).await.0, 401);
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/admin/accounts/disable",
            &owner,
            json!({"user_id":target,"disabled":false})
        )
        .await
        .0,
        200
    );
    let (_, _, fresh) = login(&app, "member@example.com", new_password).await;
    // Same operator domain operation used by recover-account, with SMTP unavailable.
    let mut tx = convex::accounts::transaction(&pool).await.unwrap();
    let token = convex::accounts::recovery_token(&mut tx, target.parse().unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let (a, b) = tokio::join!(
        call(
            &app,
            "POST",
            "/api/v1/auth/reset",
            "",
            json!({"token":token,"password":PASSWORD})
        ),
        call(
            &app,
            "POST",
            "/api/v1/auth/reset",
            "",
            json!({"token":token,"password":new_password})
        )
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [200, 400]);
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &fresh, Value::Null)
            .await
            .0,
        401
    );
    let (_, _, fresh) = login(
        &app,
        "member@example.com",
        if a.0 == 200 { PASSWORD } else { new_password },
    )
    .await;
    let (_, sessions, _) = call(&app, "GET", "/api/v1/auth/sessions", &fresh, Value::Null).await;
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/sessions/revoke",
            &fresh,
            json!({"session_id":sessions[0]["id"]})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &fresh, Value::Null)
            .await
            .0,
        401
    );
    let (_, _, fresh) = login(
        &app,
        "member@example.com",
        if a.0 == 200 { PASSWORD } else { new_password },
    )
    .await;
    assert_eq!(
        call(&app, "POST", "/api/v1/auth/logout", &fresh, Value::Null)
            .await
            .0,
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &fresh, Value::Null)
            .await
            .0,
        401
    );
    let (_, missing, _) = call(
        &app,
        "POST",
        "/api/v1/auth/recovery",
        "",
        json!({"email":"unknown@example.com"}),
    )
    .await;
    let (_, known, _) = call(
        &app,
        "POST",
        "/api/v1/auth/recovery",
        "",
        json!({"email":"member@example.com"}),
    )
    .await;
    assert_eq!(missing, known);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.recovery_tokens")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL and loopback SMTP; run mise run test:rust:integration"]
async fn configured_smtp_delivers_invitation_and_non_enumerating_recovery(pool: PgPool) {
    use lettre::{AsyncSmtpTransport, Tokio1Executor};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<String>(4);
    let smtp = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let sender = sender.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);
                writer.write_all(b"220 localhost ESMTP\r\n").await.unwrap();
                let mut data = false;
                let mut message = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap() == 0 {
                        break;
                    }
                    let reply = if data {
                        if line == ".\r\n" {
                            data = false;
                            sender.send(message.clone()).await.unwrap();
                            message.clear();
                            "250 Message accepted\r\n"
                        } else {
                            message.push_str(&line);
                            continue;
                        }
                    } else if line.starts_with("EHLO") {
                        "250-localhost\r\n250 8BITMIME\r\n"
                    } else if line.starts_with("DATA") {
                        data = true;
                        "354 End with dot\r\n"
                    } else if line.starts_with("QUIT") {
                        writer.write_all(b"221 Bye\r\n").await.unwrap();
                        break;
                    } else {
                        "250 OK\r\n"
                    };
                    writer.write_all(reply.as_bytes()).await.unwrap();
                }
            });
        }
    });
    let mut config = SecurityConfig::development();
    config.setup_token = Some(SETUP_TOKEN.into());
    config.mail = Some(std::sync::Arc::new(convex::security::Mail {
        transport: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(port)
            .build(),
        from: "convex@example.com".parse().unwrap(),
    }));
    let app = http::router_with_config(pool.clone(), config);
    let (owner, space, _) = initialized(&app).await;
    let (status, body, _) = call(
        &app,
        "POST",
        &format!("/api/v1/workspaces/{space}/invitations"),
        &owner,
        json!({"role":"member","deliver_to":"teammate@example.com"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["email_delivered"], true);
    let message = tokio::time::timeout(std::time::Duration::from_secs(3), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(message.contains("teammate@example.com"));
    assert!(message.contains("/invite#"));
    let (_, known, _) = call(
        &app,
        "POST",
        "/api/v1/auth/recovery",
        "",
        json!({"email":"owner@example.com"}),
    )
    .await;
    let message = tokio::time::timeout(std::time::Duration::from_secs(3), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    // Lettre may fold the plain text using quoted-printable. Token alphabet remains unchanged.
    let body = message.replace("=\r\n", "");
    let token = body
        .split("/reset#")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    let (_, unknown, _) = call(
        &app,
        "POST",
        "/api/v1/auth/recovery",
        "",
        json!({"email":"missing@example.com"}),
    )
    .await;
    assert_eq!(known, unknown);
    assert!(receiver.try_recv().is_err());
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/auth/reset",
            "",
            json!({"token":token,"password":"the newly recovered password"})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(
        login(&app, "owner@example.com", "the newly recovered password")
            .await
            .0,
        200
    );
    smtp.abort();
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn throttling_expiration_and_hash_only_tokens(pool: PgPool) {
    let app = app(&pool);
    let (owner, _, _) = initialized(&app).await;
    for _ in 0..10 {
        assert_eq!(login(&app, "missing@example.com", PASSWORD).await.0, 401);
    }
    assert_eq!(login(&app, "missing@example.com", PASSWORD).await.0, 429);
    let stored: String = sqlx::query_scalar("SELECT token_hash FROM convex.sessions LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, owner.strip_prefix("convex_session=").unwrap());
    sqlx::query("UPDATE convex.sessions SET expires_at=now()-interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        401
    );
}
