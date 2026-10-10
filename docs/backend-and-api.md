# Convex backend and OpenAPI contract

The backend is Rust with Axum, PostgreSQL, and SQLx. utoipa and utoipa-axum generate the OpenAPI specification from Rust response types and annotated handlers alongside route registration. The frontend uses React and TypeScript, with generated contract types and CI checks to keep artifacts current. The CP01 decisions below establish the foundation; conventions for accounts and issue operations remain proposals until their checkpoints.

## Developer tooling

Use Mise and Docker Compose almost exclusively for developer tooling. This is an agreed project convention.

- Mise owns pinned toolchain versions and the standard task entry points for setup, development, formatting, linting, builds, tests, migrations, and contract generation.
- Docker Compose owns local services and dependencies, including PostgreSQL and the development/test identity provider when OIDC work begins. Mise tasks orchestrate Compose where needed.
- Document Mise tasks as the normal development workflow. Keep CI aligned with those tasks so checks and generated artifacts follow the same workflow locally and in automation.
- Minimize prerequisites and separate task runners. Document any necessary exception and its reason rather than introducing a parallel tooling workflow.

The exact Mise tasks and Compose service layout will be established in CP01. This convention does not require every compiler or frontend process to run inside a container.

## Application structure

Build one Rust application with domain crates for accounts/authentication,
workspaces/teams and issues. Domain crates own models, rules, repository traits and
feature-gated PostgreSQL/mock adapters. Application operations coordinate those
repositories through a shared unit of work; HTTP handlers translate requests and
results. `convex` only configures dependencies, starts the server and runs operator commands.
`convex-api` owns Axum, DTOs, cookies and OpenAPI; `convex-auth` owns authentication
internals. See [backend crate boundaries](backend-crates.md) for the reading map
and transaction guarantees.

Use explicit HTTP request/response types distinct from database rows. This keeps internal credentials, token hashes, instance permissions, and persistence details out of issue responses. Derive utoipa schemas on these public types and describe documented success and error responses on handlers.

Compose the documented handlers with `utoipa-axum` so route registration and OpenAPI collection happen together. This integration supports Axum routing and collection of annotated handler documentation. It does not replace runtime validation or permission enforcement. [utoipa documentation](https://docs.rs/utoipa/latest/utoipa/), [utoipa-axum documentation](https://docs.rs/utoipa-axum/latest/utoipa_axum/)

## Agreed contract workflow

1. Define Rust HTTP types and handler documentation.
2. Compose the application router and derive its OpenAPI document.
3. Export a deterministic `openapi.json` through a command that needs no running database or identity provider. SQLx compile-time query metadata must support builds without a live database.
4. Generate TypeScript contract types from that document. Choose a generator compatible with the OpenAPI version emitted by the selected utoipa release.
5. Use those types through a shared frontend HTTP client.
6. In CI, regenerate the specification and TypeScript output and check for uncommitted differences. Verify representative live responses against the documented contract and ensure public routes are documented.

The Rust annotations and HTTP types are the contract source. Exported JSON and TypeScript are generated artifacts, with no independent manual edits. Generated types describe transport shapes; frontend view state and editor drafts remain separate.

## Initial HTTP conventions

- Use a versioned application prefix, proposed as `/api/v1`.
- Serve the specification at a stable endpoint, proposed as `/api/openapi.json`, and include an exported copy in release artifacts.
- Give operations stable, unique IDs so frontend call sites can survive route reorganization.
- Define one error format with a machine-readable code, readable message, and optional field errors. Distinguish validation, authentication, authorization, not-found, and update-conflict responses.
- Document actual browser cookie-session authentication for protected operations. OIDC is a way to obtain a Convex session; provider access tokens do not automatically authenticate application requests.
- Document public authentication routes, redirects, and invitation acceptance separately from protected issue operations. Cookie auth documentation must also describe the CSRF mechanism once selected.
- Define pagination, sort tie-breakers, filter encoding, UUID/timestamp formats, priority values, and status categories consistently across endpoints.
- For issue patches, distinguish an omitted property from an explicit null, such as leaving an assignee unchanged versus clearing it. Document expected-version behavior and the conflict response.

## First implementation checkpoint

See [CP01 Application foundation and API contract](checkpoints.md#cp01-application-foundation-and-api-contract) for the canonical deliverables and completion criteria. Every new public handler includes its OpenAPI documentation in the same change throughout the later checkpoints.

## Choices still open

- Interactive API documentation UI and its deployment settings.
- Exact cookie/CSRF contract.
- Additional error codes, pagination, and domain-operation conventions as those routes are implemented.

## CP01 implementation decisions

- **Packaging:** a top-level Cargo workspace and one application crate, `crates/convex`. Domain modules can grow inside that crate; extra crates need a concrete boundary. Frontend sources and build output stay in `web/`. The opt-in `embed-web` feature embeds `web/dist` with rust-embed, including in debug builds. The build script tracks asset changes and requires a built `index.html`. Mise bundled builds and the production Dockerfile build the frontend before Rust; deployment needs the binary and PostgreSQL. Full deployment verification remains part of CP08.
- **Frontend:** Vite builds the React SPA; TanStack Router provides browser-history routing. Bundled Axum builds serve embedded files and send `index.html` for frontend routes. Missing assets return 404. `/api` and `/api/*` dispatch to the existing API router with its JSON errors, CSRF checks, and `no-store` policy; frontend responses use separate handling, HTML revalidation, and immutable caching for Vite's hashed assets. The frontend uses relative API URLs and `CONVEX_PUBLIC_URL` remains a runtime backend setting. Development retains Vite's API proxy.
- **Contract:** utoipa 5.5 and utoipa-axum 0.2 integrate with Axum 0.8. Export through `export-openapi`, without reading application configuration or opening a database. The output is OpenAPI 3.1. `openapi-typescript` 7 generates types, and `openapi-fetch` provides the shared typed HTTP client. TypeScript 5.9 satisfies the generator's peer dependency. Cargo and pnpm lockfiles record the resolved dependency graph; Mise pins the language toolchains.
- **HTTP:** application routes use `/api/v1`, stable operation IDs, JSON bodies, and snake_case fields and enum values. `GET /api/v1/health` (`getHealth`) returns `{ "status": "ready", "version": "0.1.0" }` when PostgreSQL is reachable and the application namespace exists. It returns 503 otherwise. `GET /api/openapi.json` (`getOpenApi`) serves the same contract as offline export. Both routes are public. Unknown routes and unsupported methods return JSON errors with 404 and 405 respectively.
- **Errors:** `{ "code": "service_unavailable", "message": "…", "field_errors": { "field": ["…"] } }`. Field errors are optional and omitted when absent. The initial codes are `service_unavailable`, `not_found`, and `method_not_allowed`; account/domain checkpoints add their needed codes. Internal database errors appear in server logs, not response bodies.
- **Database:** Compose pins PostgreSQL 18.3. The first embedded SQLx migration establishes the `convex` namespace without committing to account or issue tables. Startup and `db:migrate` apply pending migrations before serving; migration failure prevents startup. Existing migrations are immutable. The build script tracks migration-directory changes so added migrations are embedded on rebuild.
- **Queries:** the initial readiness query uses SQLx's runtime query API and needs no `.sqlx` metadata. `SQLX_OFFLINE=true` keeps database-free compilation explicit. Introducing `query!`/`query_as!` requires checked-in metadata and Mise tasks to prepare and verify it in the same change.
- **Namespaces:** connections explicitly use `search_path=public` for SQLx's migration ledger, including when the PostgreSQL role is named `convex`. Domain queries must qualify tables with `convex.`. This prevents creating the application schema from changing which migration ledger a later connection resolves.
- **Testing:** router tests belong to the hermetic suite, run without a database, validate error payloads against the generated schemas, and compare the served specification to offline generation. Service-dependent targets are explicitly registered with `integration_` names and an `integration` feature requirement; their tests are ignored unless an integration task explicitly runs them. SQLx integration tests create isolated databases and verify empty-database migration, repeat migration, healthy payloads, and missing-schema readiness. Mise pins Nextest; complementary binary-name filters split the suites locally and in CI. Doctests run separately through Cargo. Playwright exercises real browser requests and retry behavior on desktop and mobile Chromium.
- **Configuration and logging:** Mise loads optional root `.env` overrides with documented local defaults. Direct binary execution needs `DATABASE_URL`; `CONVEX_BIND_ADDRESS` defaults to loopback port 3000. Tracing emits JSON with a `RUST_LOG` filter, and HTTP tracing records request outcomes. SIGINT/SIGTERM stop accepting requests, drain active requests, and close the pool.
- **CI:** the local `setup-repo` composite action installs tools/dependencies and restores caches. Separate Rust and Web jobs run the language-specific checks and tests. Rust checks, builds, unit tests, doctests, and archives enable `integration` without `embed-web` and need no frontend dependencies or assets. Rust Build runs `test:rust:archive` to build the API application and service-dependent test binaries once, then uploads the Nextest archive. Independent Rust Unit Tests and Rust Integration Tests jobs depend on that build, download the same archive, and use `test:rust:unit:from-archive` and `test:rust:integration:from-archive` without recompilation. Only the integration job starts PostgreSQL; both jobs retain JUnit reports. A separate bundle job runs `check:bundle`: it builds the real frontend and bundled application, lints all Cargo features, and verifies embedded-frontend routing. Rust checks also run doctests. Dedicated jobs run contract consistency and browser-to-API tests. Web-only jobs disable Rust caching/component setup. The aggregate `check`, `build`, and `test` tasks call the same underlying tasks locally; `build` produces a bundle, while bundle verification also has its own task. Contract checks regenerate and compare bytes, making them usable without Git metadata and ensuring missing/stale artifacts fail the check.

Docker and Compose are host prerequisites because Mise does not install a container daemon. Playwright installs Chromium and its OS libraries through a Mise task because Node cannot supply browser system dependencies; its tests emulate a mobile viewport in Chromium, not a native Safari run. Cargo and pnpm are implementation commands behind Mise tasks, rather than separate documented workflows.
