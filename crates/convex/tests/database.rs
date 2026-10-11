use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use convex::{MIGRATOR, runtime as http};
use convex_ids::{
    IssueId, MembershipId, OidcFlowId, OidcIdentityId, SessionId, UserId, WorkflowStatusId,
    WorkspaceId,
};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;

// Dropping a router closes its dedicated socket, but PostgreSQL may not have
// observed the close yet. Configuration-change tests must wait for shutdown,
// rather than racing the backend's release of its shared policy lease.
async fn wait_for_policy_leases(pool: &PgPool) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let released: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND classid=4413016::oid AND objid=3::oid AND mode='ShareLock' AND granted)").fetch_one(pool).await.unwrap();
            if released { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.expect("stopped replica releases its policy lease");
}

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
    assert_eq!(applied, 8);

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
    let contract = serde_json::to_value(convex_api::openapi()).unwrap();
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

use convex_auth::security::SecurityConfig;

const ORIGIN: &str = "http://127.0.0.1:5173";
const SETUP_TOKEN: &str = "an-installer-authorized-setup-token-for-testing";
const PASSWORD: &str = "a very strong test password";

fn app(pool: &PgPool) -> axum::Router {
    let mut config = SecurityConfig::development();
    config.setup_token = Some(SETUP_TOKEN.into());
    http::router_with_config(pool.clone(), config)
}
fn setup_request() -> Value {
    json!({"setup_token":SETUP_TOKEN,"email":"Owner@Example.com","password":PASSWORD,"display_name":"Owner"})
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
    let contract = serde_json::to_value(convex_api::openapi()).unwrap();
    let matched = contract["paths"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(pattern, _)| {
            let a: Vec<_> = pattern.split('/').collect();
            let b: Vec<_> = path.split('?').next().unwrap().split('/').collect();
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
    assert_eq!(account["workspaces"], json!([]));
    let (status, workspace, _) = call(
        app,
        "POST",
        "/api/v1/workspaces",
        &cookie,
        json!({"name":"Acme","slug":"acme","team_name":"Engineering","team_key":"eng"}),
    )
    .await;
    assert_eq!(status, 200, "{workspace}");
    assert_eq!(workspace["slug"], "acme");
    (
        cookie,
        workspace["id"].as_str().unwrap().into(),
        user["id"].as_str().unwrap().into(),
    )
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn workspace_slugs_are_validated_unique_and_stable(pool: PgPool) {
    let app = app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let invalid_workspace =
        json!({"name":"Other","slug":"invalid/slug","team_name":"Other","team_key":"OPS"});
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/workspaces",
            &owner,
            invalid_workspace
        )
        .await
        .0,
        400
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM convex.workspaces")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
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
            .bind(WorkspaceId::new())
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
    let rows: Vec<(WorkspaceId, String, String)> =
        sqlx::query_as("SELECT id,name,slug FROM convex.workspaces")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 4);
    for (id, _, slug) in &rows {
        assert_eq!(
            convex_workspaces::validation::workspace_slug(slug).unwrap(),
            *slug
        );
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
        0
    );
    let (_, account, _) = call(&app, "GET", "/api/v1/auth/me", &cookie, Value::Null).await;
    assert_eq!(account["user"]["is_instance_admin"], true);
    assert_eq!(account["workspaces"], json!([]));
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
            .bind(member_id.as_str().unwrap().parse::<MembershipId>().unwrap())
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
        200
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
        .bind(convex_auth::security::digest(&revoked))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(join(&app, &revoked, "third@example.com").await.0, 400);
    let expired = invitation(&app, &owner, &space, "member").await;
    sqlx::query(
        "UPDATE convex.invitations SET expires_at=now()-interval '1 second' WHERE token_hash=$1",
    )
    .bind(convex_auth::security::digest(&expired))
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
    let mut tx = convex_auth::postgres::accounts::transaction(&pool)
        .await
        .unwrap();
    let token = convex_auth::postgres::accounts::recovery_token(&mut tx, target.parse().unwrap())
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

async fn smtp_inbox() -> (
    std::sync::Arc<convex_auth::security::Mail>,
    tokio::sync::mpsc::Receiver<String>,
    tokio::task::JoinHandle<()>,
) {
    use lettre::{AsyncSmtpTransport, Tokio1Executor};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sender, receiver) = tokio::sync::mpsc::channel::<String>(4);
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
    let mail = std::sync::Arc::new(convex_auth::security::Mail {
        transport: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(port)
            .build(),
        from: "convex@example.com".parse().unwrap(),
    });
    (mail, receiver, smtp)
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL and loopback SMTP; run mise run test:rust:integration"]
async fn configured_smtp_delivers_invitation_and_non_enumerating_recovery(pool: PgPool) {
    let (mail, mut receiver, smtp) = smtp_inbox().await;
    let mut config = SecurityConfig::development();
    config.setup_token = Some(SETUP_TOKEN.into());
    config.mail = Some(mail);
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

fn policy_config(ttl: &str, provider: bool, required: bool) -> SecurityConfig {
    let mut config = SecurityConfig::development();
    config.setup_token = Some(SETUP_TOKEN.into());
    config.authentication.session_lifetime =
        convex_auth::auth_policy::SessionLifetime::parse(ttl).unwrap();
    config.authentication.oidc_required = required;
    if provider {
        config.authentication.provider = Some(convex_auth::auth_policy::ProviderConfig {
            issuer: "http://127.0.0.1:1/realms/convex".into(),
            client_id: "convex".into(),
            client_secret: "test-secret".into(),
            display_name: "Test provider".into(),
            enabled: true,
        });
    }
    config
}

// This seeds a validated identity at the domain interface. It deliberately does not
// simulate protocol validation: login/callback interoperability belongs to the next slice.
async fn oidc_account(pool: &PgPool, admin: bool) -> (UserId, OidcIdentityId, String) {
    use convex_auth::{postgres::accounts, security};
    let user = UserId::new();
    let identity = OidcIdentityId::new();
    let mut tx = accounts::transaction(pool).await.unwrap();
    sqlx::query("INSERT INTO convex.users (id,display_name,is_instance_admin) VALUES ($1,'External user',$2)")
        .bind(user).bind(admin).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO convex.oidc_identities (id,user_id,issuer,subject) VALUES ($1,$2,'http://127.0.0.1:1/realms/convex',$3)")
        .bind(identity).bind(user).bind(user.to_string()).execute(&mut *tx).await.unwrap();
    let token = convex_auth::postgres::security::new_session(
        &mut tx,
        user,
        security::LoginMethod::Oidc(identity),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (user, identity, format!("convex_session={token}"))
}

async fn session_expiry(pool: &PgPool, cookie: &str) -> chrono::DateTime<chrono::Utc> {
    sqlx::query_scalar("SELECT expires_at FROM convex.sessions WHERE token_hash=$1")
        .bind(convex_auth::security::digest(
            cookie.strip_prefix("convex_session=").unwrap(),
        ))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn shared_session_lifetime_oidc_invitation_and_admin_visibility(pool: PgPool) {
    let config = policy_config("10800", true, false);
    let cookie_header =
        convex_api::security::CookieConfig::from(&config).session_cookie("a-valid-test-token");
    assert!(cookie_header.to_str().unwrap().contains("Max-Age=10800"));
    assert!(
        convex_api::security::CookieConfig::from(&config)
            .session_cookie("")
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    let app = http::prepare_app(pool.clone(), config).await.unwrap();
    let (owner, space, _) = initialized(&app).await;
    let (external_user, identity, external) = oidc_account(&pool, false).await;
    let tokens = [owner.clone(), external.clone()];
    for cookie in tokens {
        let seconds: f64 = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM expires_at-created_at)::double precision FROM convex.sessions WHERE token_hash=$1")
            .bind(convex_auth::security::digest(cookie.strip_prefix("convex_session=").unwrap()))
            .fetch_one(&pool).await.unwrap();
        assert_eq!(seconds, 10800.0);
        assert_eq!(
            call(&app, "GET", "/api/v1/auth/me", &cookie, Value::Null)
                .await
                .0,
            200
        );
    }
    let invite = invitation(&app, &owner, &space, "member").await;
    let (status, _, rotated) = call(
        &app,
        "POST",
        "/api/v1/invitations/accept",
        &external,
        json!({"token":invite}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &external, Value::Null)
            .await
            .0,
        401
    );
    let provenance: (String, OidcIdentityId, f64) = sqlx::query_as("SELECT login_method,oidc_identity_id,EXTRACT(EPOCH FROM expires_at-created_at)::double precision FROM convex.sessions WHERE token_hash=$1")
        .bind(convex_auth::security::digest(rotated.strip_prefix("convex_session=").unwrap()))
        .fetch_one(&pool).await.unwrap();
    assert_eq!(provenance, ("oidc".into(), identity, 10800.0));
    let (_, me, _) = call(&app, "GET", "/api/v1/auth/me", &rotated, Value::Null).await;
    assert_eq!(me["user"]["id"], external_user.to_string());
    assert_eq!(me["workspaces"][0]["id"], space);
    let (status, list, _) = call(&app, "GET", "/api/v1/admin/accounts", &owner, Value::Null).await;
    assert_eq!(status, 200);
    let found = list
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == external_user.to_string())
        .unwrap();
    assert!(found["email"].is_null());
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/admin/accounts/disable",
            &owner,
            json!({"user_id":external_user,"disabled":true})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &rotated, Value::Null)
            .await
            .0,
        401
    );
    let mut tx = convex_auth::postgres::accounts::transaction(&pool)
        .await
        .unwrap();
    assert!(
        convex_auth::postgres::security::new_session(
            &mut tx,
            external_user,
            convex_auth::security::LoginMethod::Oidc(identity)
        )
        .await
        .is_err()
    );
    tx.rollback().await.unwrap();
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn shortening_sessions_is_persistent_and_increases_never_resurrect(pool: PgPool) {
    let app = http::prepare_app(pool.clone(), policy_config("604800", true, false))
        .await
        .unwrap();
    initialized(&app).await;
    let (_, _, young) = login(&app, "owner@example.com", PASSWORD).await;
    // Use another live session rather than the one local login rotated away.
    let mut tx = convex_auth::postgres::accounts::transaction(&pool)
        .await
        .unwrap();
    let user: UserId = sqlx::query_scalar("SELECT id FROM convex.users LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let old_token = convex_auth::postgres::security::new_session(
        &mut tx,
        user,
        convex_auth::security::LoginMethod::Local,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let old = format!("convex_session={old_token}");
    let (_, _, oidc) = oidc_account(&pool, false).await;
    sqlx::query("UPDATE convex.sessions SET created_at=now()-interval '2 days',expires_at=now()+interval '5 days' WHERE token_hash=$1")
        .bind(convex_auth::security::digest(&old_token)).execute(&pool).await.unwrap();
    drop(app);
    wait_for_policy_leases(&pool).await;
    let shorter = http::prepare_app(pool.clone(), policy_config("86400", true, false))
        .await
        .unwrap();
    assert_eq!(
        call(&shorter, "GET", "/api/v1/auth/me", &old, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(
        call(&shorter, "GET", "/api/v1/auth/me", &young, Value::Null)
            .await
            .0,
        200
    );
    assert_eq!(
        call(&shorter, "GET", "/api/v1/auth/me", &oidc, Value::Null)
            .await
            .0,
        200
    );
    let young_expiry = session_expiry(&pool, &young).await;
    let oidc_expiry = session_expiry(&pool, &oidc).await;
    let old_expiry = session_expiry(&pool, &old).await;
    let duration: f64 = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM expires_at-created_at)::double precision FROM convex.sessions WHERE token_hash=$1")
        .bind(convex_auth::security::digest(&old_token)).fetch_one(&pool).await.unwrap();
    assert_eq!(duration, 86400.0);
    // Activity does not slide expiration forward.
    assert_eq!(session_expiry(&pool, &young).await, young_expiry);
    drop(shorter);
    wait_for_policy_leases(&pool).await;
    let longer = http::prepare_app(pool.clone(), policy_config("604800", true, false))
        .await
        .unwrap();
    assert_eq!(
        call(&longer, "GET", "/api/v1/auth/me", &old, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(session_expiry(&pool, &old).await, old_expiry);
    assert_eq!(session_expiry(&pool, &young).await, young_expiry);
    assert_eq!(session_expiry(&pool, &oidc).await, oidc_expiry);
    let (status, _, new) = login(&longer, "owner@example.com", PASSWORD).await;
    assert_eq!(status, 200);
    let duration: f64 = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM expires_at-created_at)::double precision FROM convex.sessions WHERE token_hash=$1")
        .bind(convex_auth::security::digest(new.strip_prefix("convex_session=").unwrap())).fetch_one(&pool).await.unwrap();
    assert_eq!(duration, 604800.0);
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn required_oidc_revokes_ordinary_local_access_and_preserves_admin_recovery(pool: PgPool) {
    let app = http::prepare_app(pool.clone(), policy_config("604800", true, false))
        .await
        .unwrap();
    let (owner, space, owner_id) = initialized(&app).await;
    let invite = invitation(&app, &owner, &space, "member").await;
    let (status, _, local) = join(&app, &invite, "person@example.com").await;
    assert_eq!(status, 200);
    let (_, me, _) = call(&app, "GET", "/api/v1/auth/me", &local, Value::Null).await;
    let ordinary: UserId = serde_json::from_value(me["user"]["id"].clone()).unwrap();
    let mut tx = convex_auth::postgres::accounts::transaction(&pool)
        .await
        .unwrap();
    let old_reset = convex_auth::postgres::accounts::recovery_token(&mut tx, ordinary)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    drop(app);
    wait_for_policy_leases(&pool).await;
    let required = http::prepare_app(pool.clone(), policy_config("604800", true, true))
        .await
        .unwrap();
    assert_eq!(
        call(&required, "GET", "/api/v1/auth/me", &local, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(
        call(&required, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        200
    );
    assert_eq!(
        login(&required, "person@example.com", PASSWORD).await.0,
        403
    );
    let fresh_invite = invitation(&required, &owner, &space, "member").await;
    assert_eq!(
        join(&required, &fresh_invite, "new@example.com").await.0,
        403
    );
    assert_eq!(
        call(
            &required,
            "POST",
            "/api/v1/auth/reset",
            "",
            json!({"token":old_reset,"password":PASSWORD})
        )
        .await
        .0,
        403
    );
    assert_eq!(
        call(
            &required,
            "POST",
            "/api/v1/auth/recovery",
            "",
            json!({"email":"person@example.com"})
        )
        .await
        .0,
        200
    );
    let mut tx = convex_auth::postgres::accounts::transaction(&pool)
        .await
        .unwrap();
    assert!(
        convex_auth::postgres::accounts::recovery_token(&mut tx, ordinary)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let (_, _, oidc_admin) = oidc_account(&pool, true).await;
    let (status, body, _) = call(
        &required,
        "POST",
        "/api/v1/admin/accounts/disable",
        &oidc_admin,
        json!({"user_id":owner_id,"disabled":true}),
    )
    .await;
    assert_eq!(status, 409);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("local credentials")
    );
    let mut tx = convex_auth::postgres::accounts::transaction(&pool)
        .await
        .unwrap();
    let reset = convex_auth::postgres::accounts::recovery_token(&mut tx, owner_id.parse().unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        call(
            &required,
            "POST",
            "/api/v1/auth/reset",
            "",
            json!({"token":reset,"password":PASSWORD})
        )
        .await
        .0,
        200
    );
    assert_eq!(login(&required, "owner@example.com", PASSWORD).await.0, 200);
    // Disabling enforcement does not restore sessions explicitly revoked by its activation.
    drop(required);
    wait_for_policy_leases(&pool).await;
    let optional = http::prepare_app(pool.clone(), policy_config("604800", true, false))
        .await
        .unwrap();
    assert_eq!(
        call(&optional, "GET", "/api/v1/auth/me", &local, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(
        login(&optional, "person@example.com", PASSWORD).await.0,
        200
    );
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn provider_secret_rotation_preserves_sessions_but_client_change_revokes(pool: PgPool) {
    let config = policy_config("604800", true, false);
    let app = http::prepare_app(pool.clone(), config.clone())
        .await
        .unwrap();
    let (owner, _, _) = initialized(&app).await;
    let (_, _, external) = oidc_account(&pool, false).await;
    sqlx::query("INSERT INTO convex.oidc_flows (id,state_hash,browser_hash,intent,exchange_revision,nonce,pkce_verifier) VALUES ($1,'hashed-state','hashed-browser','login',$2,'nonce','verifier')")
        .bind(OidcFlowId::new()).bind(config.authentication.exchange_revision()).execute(&pool).await.unwrap();
    drop(app);
    wait_for_policy_leases(&pool).await;
    let mut rotated = config;
    rotated
        .authentication
        .provider
        .as_mut()
        .unwrap()
        .client_secret = "rotated secret".into();
    let app = http::prepare_app(pool.clone(), rotated.clone())
        .await
        .unwrap();
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &external, Value::Null)
            .await
            .0,
        200
    );
    let flows: i64 = sqlx::query_scalar("SELECT count(*) FROM convex.oidc_flows")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(flows, 0);
    // Convex-only logout needs no request to the unreachable provider.
    assert_eq!(
        call(&app, "POST", "/api/v1/auth/logout", &external, json!({}))
            .await
            .0,
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &external, Value::Null)
            .await
            .0,
        401
    );
    let (_, _, another) = oidc_account(&pool, false).await;
    drop(app);
    wait_for_policy_leases(&pool).await;
    rotated.authentication.provider.as_mut().unwrap().client_id = "different client".into();
    let app = http::prepare_app(pool.clone(), rotated.clone())
        .await
        .unwrap();
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &another, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        200
    );
    let (_, _, disabled) = oidc_account(&pool, false).await;
    drop(app);
    wait_for_policy_leases(&pool).await;
    rotated.authentication.provider.as_mut().unwrap().enabled = false;
    let app = http::prepare_app(pool.clone(), rotated).await.unwrap();
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &disabled, Value::Null)
            .await
            .0,
        401
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        200
    );
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn authentication_replicas_must_agree_and_lost_leases_fail_closed(pool: PgPool) {
    let config = policy_config("604800", false, false);
    let first = http::prepare_app(pool.clone(), config.clone())
        .await
        .unwrap();
    let (owner, _, _) = initialized(&first).await;
    let same = http::prepare_app(pool.clone(), config).await.unwrap();
    assert_eq!(
        call(&same, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        200
    );
    let changed = policy_config("3600", false, false);
    let error = http::prepare_app(pool.clone(), changed.clone())
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains("Stop all replicas"));
    drop(same);
    assert!(
        http::prepare_app(pool.clone(), changed.clone())
            .await
            .is_err()
    );
    // Terminate this test database's dedicated shared lease connection. A failed
    // replica may not reconnect and overwrite a policy activated by its replacement.
    let terminated: Vec<bool> = sqlx::query_scalar("SELECT pg_terminate_backend(pid) FROM pg_locks WHERE locktype='advisory' AND database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND classid=$1::bigint::oid AND objid=$2::bigint::oid AND mode='ShareLock'")
        .bind(0x435658_i64).bind(3_i64).fetch_all(&pool).await.unwrap();
    assert_eq!(terminated, vec![true]);
    assert_eq!(
        call(&first, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        503
    );
    assert_eq!(
        call(&first, "GET", "/api/v1/health", "", Value::Null)
            .await
            .0,
        503
    );
    wait_for_policy_leases(&pool).await;
    let replacement = http::prepare_app(pool.clone(), changed).await.unwrap();
    assert_eq!(
        call(&replacement, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        200
    );
    assert_eq!(
        call(&first, "GET", "/api/v1/auth/me", &owner, Value::Null)
            .await
            .0,
        503
    );
}

#[sqlx::test(migrations = false)]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn cp02_session_upgrade_preserves_local_authentication(pool: PgPool) {
    for sql in [
        include_str!("../../../migrations/202610080001_application_schema.sql"),
        include_str!("../../../migrations/202610080002_accounts_workspaces.sql"),
        include_str!("../../../migrations/202610080003_workspace_slugs.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let user = UserId::new();
    let token = convex_auth::security::token();
    sqlx::query("INSERT INTO convex.users (id,display_name,is_instance_admin) VALUES ($1,'Existing administrator',true)")
        .bind(user).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO convex.local_credentials (user_id,email,password_hash) VALUES ($1,'existing@example.com','existing hash')")
        .bind(user).execute(&pool).await.unwrap();
    // Exercise the old migration's session default, before the new migration exists.
    sqlx::query("INSERT INTO convex.sessions (id,user_id,token_hash,login_method) VALUES ($1,$2,$3,'local')")
        .bind(SessionId::new()).bind(user).bind(convex_auth::security::digest(&token)).execute(&pool).await.unwrap();
    let cookie = format!("convex_session={token}");
    let old_expiry = session_expiry(&pool, &cookie).await;
    sqlx::raw_sql(include_str!("../../../migrations/202610090001_issues.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/202610090002_authentication_policy.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for sql in [
        include_str!("../../../migrations/202610100001_oidc_callback_origin.sql"),
        include_str!("../../../migrations/202610100002_authentication_methods.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let app = http::prepare_app(pool.clone(), policy_config("604800", false, false))
        .await
        .unwrap();
    assert_eq!(session_expiry(&pool, &cookie).await, old_expiry);
    let (status, me, _) = call(&app, "GET", "/api/v1/auth/me", &cookie, Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(me["user"]["id"], user.to_string());
}

mod oidc_protocol;

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn issue_workflow_concurrency_isolation_and_history(pool: PgPool) {
    let app = app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let (_, workspace, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    let team = &workspace["teams"][0];
    let collection = format!("/api/v1/workspaces/{space}/issues");
    let req = json!({"team_id":team["id"],"title":" First issue ","description":"## Markdown\nWork to do"});
    let (a, b) = tokio::join!(
        call(&app, "POST", &collection, &owner, req.clone()),
        call(&app, "POST", &collection, &owner, req)
    );
    assert_eq!((a.0, b.0), (201, 201));
    assert_ne!(a.1["number"], b.1["number"]);
    assert_eq!(a.1["title"], "First issue");
    assert_eq!(a.1["status_id"], team["statuses"][1]["id"]);
    let issue = a.1;
    // A team collection excludes other teams, rejects foreign scope, and validates UUIDs.
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/teams"),
            &owner,
            json!({"name":"Design","key":"DES"})
        )
        .await
        .0,
        200
    );
    let (_, expanded, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    let design = &expanded["teams"][1];
    let (_, design_issue, _) = call(
        &app,
        "POST",
        &collection,
        &owner,
        json!({"team_id":design["id"],"title":"Design only"}),
    )
    .await;
    let team_collection = format!("{collection}?team_id={}", team["id"].as_str().unwrap());
    let (code, team_issues, _) = call(&app, "GET", &team_collection, &owner, Value::Null).await;
    assert_eq!(code, 200);
    assert_eq!(team_issues.as_array().unwrap().len(), 2);
    assert!(
        team_issues
            .as_array()
            .unwrap()
            .iter()
            .all(|issue| issue["team_id"] == team["id"])
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("{collection}?team_id={}", design["id"].as_str().unwrap()),
            &owner,
            Value::Null
        )
        .await
        .1,
        json!([design_issue])
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("{collection}?team_id=invalid"),
            &owner,
            Value::Null
        )
        .await
        .0,
        400
    );
    let path = format!("{collection}/{}", issue["id"].as_str().unwrap());
    let identifier_path = format!("{collection}/{}", issue["identifier"].as_str().unwrap());
    assert_eq!(
        call(&app, "GET", &identifier_path, &owner, Value::Null)
            .await
            .1,
        issue
    );
    let (a, b) = tokio::join!(
        call(
            &app,
            "PATCH",
            &path,
            &owner,
            json!({"expected_version":1,"title":"A"})
        ),
        call(
            &app,
            "PATCH",
            &path,
            &owner,
            json!({"expected_version":1,"title":"B"})
        )
    );
    assert!([a.0, b.0].contains(&200) && [a.0, b.0].contains(&409));
    let (winner, conflict) = if a.0 == 200 { (a.1, b.1) } else { (b.1, a.1) };
    assert_eq!(conflict["current_issue"], winner);
    for patch in [
        json!({"title":null}),
        json!({"description":null}),
        json!({"status_id":null}),
        json!({"priority":null}),
        json!({"archived":null}),
        json!({"title":"  "}),
        json!({"team_id":team["id"]}),
        json!({"status_id":WorkflowStatusId::new()}),
        json!({"assignee_membership_id":MembershipId::new()}),
        json!({"archived":true,"title":"no"}),
    ] {
        let mut patch = patch;
        patch["expected_version"] = json!(2);
        assert_eq!(call(&app, "PATCH", &path, &owner, patch).await.0, 400);
    }
    assert_eq!(
        call(
            &app,
            "PATCH",
            &path,
            &owner,
            json!({"expected_version":2,"assignee_membership_id":null})
        )
        .await
        .1["version"],
        2
    );
    // Another workspace's real status/membership/team UUIDs cannot be referenced.
    let (_, other, _) = call(
        &app,
        "POST",
        "/api/v1/workspaces",
        &owner,
        json!({"name":"Other","slug":"other","team_name":"Other","team_key":"OTH"}),
    )
    .await;
    let other_space = other["id"].as_str().unwrap();
    let (_, other, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{other_space}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        call(
            &app,
            "GET",
            &format!(
                "{collection}?team_id={}",
                other["teams"][0]["id"].as_str().unwrap()
            ),
            &owner,
            Value::Null
        )
        .await
        .0,
        404
    );
    for patch in [
        json!({"expected_version":2,"status_id":other["teams"][0]["statuses"][0]["id"]}),
        json!({"expected_version":2,"assignee_membership_id":other["members"][0]["id"]}),
    ] {
        assert_eq!(call(&app, "PATCH", &path, &owner, patch).await.0, 400);
    }
    assert_eq!(
        call(
            &app,
            "POST",
            &collection,
            &owner,
            json!({"team_id":other["teams"][0]["id"],"title":"Foreign"})
        )
        .await
        .0,
        400
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!(
                "/api/v1/workspaces/{other_space}/issues/{}",
                issue["id"].as_str().unwrap()
            ),
            &owner,
            Value::Null
        )
        .await
        .0,
        404
    );
    // Invite an ordinary member and exercise permissions through their own session.
    let (_, invite, _) = call(
        &app,
        "POST",
        &format!("/api/v1/workspaces/{space}/invitations"),
        &owner,
        json!({"role":"member"}),
    )
    .await;
    let token = invite["url"].as_str().unwrap().split('#').nth(1).unwrap();
    let (status,_,member_cookie)=call(&app,"POST","/api/v1/invitations/accept","",json!({"token":token,"email":"issue-member@example.com","password":PASSWORD,"display_name":"Issue member"})).await;
    assert_eq!(status, 200);
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/v1/workspaces/{other_space}/issues"),
            &member_cookie,
            Value::Null
        )
        .await
        .0,
        404
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &format!(
                "/api/v1/workspaces/{other_space}/issues/{}",
                issue["id"].as_str().unwrap()
            ),
            &member_cookie,
            json!({"expected_version":2,"title":"Leak"})
        )
        .await
        .0,
        404
    );
    let (_, workspace, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    let member = &workspace["members"][1]["id"];
    assert_eq!(
        call(
            &app,
            "PATCH",
            &path,
            &member_cookie,
            json!({"expected_version":2,"assignee_membership_id":member,"priority":"high"})
        )
        .await
        .1["version"],
        3
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &path,
            &owner,
            json!({"expected_version":3,"archived":true})
        )
        .await
        .1["version"],
        4
    );
    assert_eq!(
        call(&app, "GET", &team_collection, &member_cookie, Value::Null)
            .await
            .1
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        call(&app, "GET", &identifier_path, &member_cookie, Value::Null)
            .await
            .1["archived_at"]
            .is_string()
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &path,
            &member_cookie,
            json!({"expected_version":4,"title":"Archived edit"})
        )
        .await
        .0,
        400
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("{team_collection}&archived=true"),
            &member_cookie,
            Value::Null
        )
        .await
        .1
        .as_array()
        .unwrap()
        .len(),
        1
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("{collection}?archived=invalid"),
            &member_cookie,
            Value::Null
        )
        .await
        .0,
        400
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!(
                "/api/v1/workspaces/{space}/teams/{}/rename",
                team["id"].as_str().unwrap()
            ),
            &owner,
            json!({"name":"Renamed team"})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(&app, "GET", &identifier_path, &member_cookie, Value::Null)
            .await
            .1["identifier"],
        issue["identifier"]
    );
    // Removing an assignee clears assignment even on an archived issue.
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/members"),
            &owner,
            json!({"membership_id":member,"remove":true})
        )
        .await
        .0,
        200
    );
    let (_, after, _) = call(&app, "GET", &path, &owner, Value::Null).await;
    assert!(after["assignee_membership_id"].is_null());
    assert_eq!(after["version"], 5);
    assert_eq!(
        call(
            &app,
            "PATCH",
            &path,
            &owner,
            json!({"expected_version":5,"archived":false})
        )
        .await
        .1["version"],
        6
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &path,
            &owner,
            json!({"expected_version":6,"assignee_membership_id":member})
        )
        .await
        .0,
        400
    );
    let (_, after, _) = call(&app, "GET", &path, &owner, Value::Null).await;
    assert_eq!(after["description"], issue["description"]);
    assert_eq!(
        after["creator_membership_id"],
        issue["creator_membership_id"]
    );
    let payloads: Vec<Value> = sqlx::query_scalar(
        "SELECT payload FROM convex.issue_activity WHERE issue_id=$1 ORDER BY created_at,id",
    )
    .bind(issue["id"].as_str().unwrap().parse::<IssueId>().unwrap())
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(payloads.len(), 6);
    assert_eq!(payloads[4]["before"]["assignee"]["name"], "Issue member");
    assert_eq!(payloads[4]["actor_name"], "Owner");
    assert!(payloads[4]["after"]["assignee"]["id"].is_null());
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn issue_activity_failure_rolls_back_mutation_and_counter(pool: PgPool) {
    let app = app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let (_, workspace, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    let collection = format!("/api/v1/workspaces/{space}/issues");
    let req = json!({"team_id":workspace["teams"][0]["id"],"title":"Rollback"});
    let (_, issue, _) = call(&app, "POST", &collection, &owner, req.clone()).await;
    let path = format!("{collection}/{}", issue["id"].as_str().unwrap());
    sqlx::raw_sql("CREATE FUNCTION convex.reject_activity() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test activity failure'; END $$; CREATE TRIGGER reject_activity BEFORE INSERT ON convex.issue_activity FOR EACH ROW EXECUTE FUNCTION convex.reject_activity();").execute(&pool).await.unwrap();
    assert_eq!(
        call(
            &app,
            "PATCH",
            &path,
            &owner,
            json!({"expected_version":1,"title":"Must roll back"})
        )
        .await
        .0,
        503
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &path,
            &owner,
            json!({"expected_version":1,"archived":true})
        )
        .await
        .0,
        503
    );
    assert_eq!(
        call(&app, "POST", &collection, &owner, req.clone()).await.0,
        503
    );
    assert_eq!(call(&app, "GET", &path, &owner, Value::Null).await.1, issue);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM convex.issue_activity")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    sqlx::query("DROP TRIGGER reject_activity ON convex.issue_activity")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        call(&app, "POST", &collection, &owner, req).await.1["number"],
        2
    );
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL; run mise run test:rust:integration"]
async fn assigning_and_removing_a_member_coordinate_and_removal_rolls_back(pool: PgPool) {
    let app = app(&pool);
    let (owner, space, _) = initialized(&app).await;
    let space_id = space.parse::<WorkspaceId>().unwrap();
    let user = UserId::new();
    let member = MembershipId::new();
    sqlx::query("INSERT INTO convex.users(id,display_name) VALUES($1,'Departing member')")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO convex.workspace_memberships(id,workspace_id,user_id,role) VALUES($1,$2,$3,'member')").bind(member).bind(space_id).bind(user).execute(&pool).await.unwrap();
    let (_, workspace, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    let collection = format!("/api/v1/workspaces/{space}/issues");
    let (_, issue, _) = call(
        &app,
        "POST",
        &collection,
        &owner,
        json!({"team_id":workspace["teams"][0]["id"],"title":"Assignment race"}),
    )
    .await;
    let path = format!("{collection}/{}", issue["id"].as_str().unwrap());
    let remove_path = format!("/api/v1/workspaces/{space}/members");
    let race = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            call(
                &app,
                "PATCH",
                &path,
                &owner,
                json!({"expected_version":1,"assignee_membership_id":member})
            ),
            call(
                &app,
                "POST",
                &remove_path,
                &owner,
                json!({"membership_id":member,"remove":true})
            )
        )
    })
    .await
    .expect("Scoped locks must not deadlock with CP02 account locks");
    assert_eq!(race.1.0, 200);
    assert!([200, 400].contains(&race.0.0));
    let (_, after, _) = call(&app, "GET", &path, &owner, Value::Null).await;
    assert!(after["assignee_membership_id"].is_null());
    // Reactivate the same membership and force a failure during removal history.
    sqlx::query("UPDATE convex.workspace_memberships SET removed_at=NULL WHERE id=$1")
        .bind(member)
        .execute(&pool)
        .await
        .unwrap();
    let (_, assigned, _) = call(
        &app,
        "PATCH",
        &path,
        &owner,
        json!({"expected_version":after["version"],"assignee_membership_id":member}),
    )
    .await;
    sqlx::raw_sql("CREATE FUNCTION convex.reject_removal_history() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test history failure'; END $$; CREATE TRIGGER reject_removal_history BEFORE INSERT ON convex.issue_activity FOR EACH ROW EXECUTE FUNCTION convex.reject_removal_history();").execute(&pool).await.unwrap();
    assert_eq!(
        call(
            &app,
            "POST",
            &remove_path,
            &owner,
            json!({"membership_id":member,"remove":true})
        )
        .await
        .0,
        503
    );
    assert_eq!(
        call(&app, "GET", &path, &owner, Value::Null).await.1,
        assigned
    );
    let active: bool = sqlx::query_scalar(
        "SELECT removed_at IS NULL FROM convex.workspace_memberships WHERE id=$1",
    )
    .bind(member)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(active);
}

#[sqlx::test(migrator = "convex::MIGRATOR")]
#[ignore = "requires PostgreSQL and loopback SMTP; run mise run test:rust:integration"]
async fn workspace_deletion_emails_bound_expiring_codes_and_removes_only_its_data(pool: PgPool) {
    let (mail, mut inbox, smtp) = smtp_inbox().await;
    let mut config = SecurityConfig::development();
    config.setup_token = Some(SETUP_TOKEN.into());
    config.mail = Some(mail);
    let app = http::router_with_config(pool.clone(), config);
    let (owner, space, _) = initialized(&app).await;
    let token = invitation(&app, &owner, &space, "member").await;
    let (_, _, member) = join(&app, &token, "deletion-member@example.com").await;
    // Ordinary members can create a workspace of their own.
    let (status, other, _) = call(
        &app,
        "POST",
        "/api/v1/workspaces",
        &member,
        json!({"name":"Other","slug":"other","team_name":"Other","team_key":"OTH"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(other["role"], "owner");
    let (_, detail, _) = call(
        &app,
        "GET",
        &format!("/api/v1/workspaces/{space}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/v1/workspaces/{space}/issues"),
            &owner,
            json!({"team_id":detail["teams"][0]["id"],"title":"Work to delete"})
        )
        .await
        .0,
        201
    );
    // An outstanding OIDC invitation flow must not keep the invitation alive.
    let invite = invitation(&app, &owner, &space, "member").await;
    sqlx::query("INSERT INTO convex.oidc_flows (id,state_hash,browser_hash,intent,exchange_revision,invitation_id)
        SELECT gen_random_uuid(),'deletion-state','browser','invite','revision',id FROM convex.invitations WHERE token_hash=$1")
        .bind(convex_auth::security::digest(&invite)).execute(&pool).await.unwrap();
    let send = format!("/api/v1/workspaces/{space}/deletion-code");
    let delete = format!("/api/v1/workspaces/{space}/delete");
    assert_eq!(call(&app, "POST", &send, &member, Value::Null).await.0, 403);
    let mut previous = String::new();
    let mut current = String::new();
    for _ in 0..2 {
        let (status, response, _) = call(&app, "POST", &send, &owner, Value::Null).await;
        assert_eq!(status, 200, "{response}");
        assert_eq!(response["email"], "owner@example.com");
        let message = tokio::time::timeout(std::time::Duration::from_secs(3), inbox.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(message.contains("owner@example.com"));
        assert!(message.contains("Acme"));
        assert!(message.contains("10 minutes"));
        previous = current;
        current = message
            .lines()
            .find(|line| line.len() == 8 && line.bytes().all(|b| b.is_ascii_hexdigit()))
            .unwrap()
            .into();
    }
    let request = |code: &str| json!({"code":code,"acknowledged":true});
    assert_eq!(
        call(&app, "POST", &delete, &owner, request(&previous))
            .await
            .0,
        400
    );
    assert_eq!(
        call(&app, "POST", &delete, &member, request(&current))
            .await
            .0,
        403
    );
    let other_delete = format!(
        "/api/v1/workspaces/{}/delete",
        other["id"].as_str().unwrap()
    );
    assert_eq!(
        call(&app, "POST", &other_delete, &member, request(&current))
            .await
            .0,
        400
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &delete,
            &owner,
            json!({"code":current,"acknowledged":false})
        )
        .await
        .0,
        400
    );
    sqlx::query("UPDATE convex.workspace_deletion_codes SET expires_at=now()-interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        call(&app, "POST", &delete, &owner, request(&current))
            .await
            .0,
        400
    );
    assert_eq!(call(&app, "POST", &send, &owner, Value::Null).await.0, 200);
    let message = tokio::time::timeout(std::time::Duration::from_secs(3), inbox.recv())
        .await
        .unwrap()
        .unwrap();
    let current = message
        .lines()
        .find(|line| line.len() == 8 && line.bytes().all(|b| b.is_ascii_hexdigit()))
        .unwrap();
    assert_eq!(
        call(&app, "POST", &delete, &owner, request(current))
            .await
            .0,
        200
    );
    assert_eq!(
        call(&app, "POST", &delete, &owner, request(current))
            .await
            .0,
        404
    );
    for table in [
        "workspaces",
        "workspace_memberships",
        "teams",
        "workflow_statuses",
        "issues",
        "issue_activity",
        "invitations",
        "workspace_deletion_codes",
    ] {
        let column = if table == "workspaces" {
            "id"
        } else {
            "workspace_id"
        };
        let count: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM convex.{table} WHERE {column}::text=$1"
        ))
        .bind(&space)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0, "{table} still has workspace data");
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM convex.oidc_flows")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let (status, account, _) = call(&app, "GET", "/api/v1/auth/me", &member, Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(account["workspaces"].as_array().unwrap().len(), 1);
    assert_eq!(account["workspaces"][0]["id"], other["id"]);
    let (status, account, _) = call(&app, "GET", "/api/v1/auth/me", &owner, Value::Null).await;
    assert_eq!(status, 200);
    assert!(account["workspaces"].as_array().unwrap().is_empty());
    smtp.abort();
}
