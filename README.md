# Convex

An open-source issue tracker, starting with issues, labels, comments, and search for self-hosting teams.

## Development

Install [Mise](https://mise.jdx.dev/getting-started.html) and Docker with Compose v2 or newer, then run these commands from the repository root:

```sh
mise trust
mise install
mise run rust-components
mise run setup
mise run dev
```

Open <http://127.0.0.1:5173>. The frontend opens first-admin onboarding, then local login and workspace settings. Configure `CONVEX_SETUP_TOKEN` before creating the first admin; see [installation and recovery](docs/authentication.md#first-installation-and-operator-recovery). The system connection screen is available at `/system`. The API listens on <http://127.0.0.1:3000> and applies migrations before serving requests. Stop development with Ctrl-C; `mise run db:down` stops the database containers while preserving development data.

Local defaults work without an `.env` file. Copy `.env.example` to `.env` to customize the database URLs, bind address, or log filter. The frontend uses Vite's `/api` proxy; `CONVEX_API_PROXY` changes its backend target. Never expose the local Compose databases outside your machine; their credentials are development defaults.

| Task | Purpose |
| --- | --- |
| `mise run dev` | Start PostgreSQL and both development processes |
| `mise run db:migrate` | Apply migrations to the configured development database |
| `mise run generate` | Export OpenAPI and regenerate TypeScript types without a database |
| `mise run check` | Check formatting, linting, types, and contract consistency |
| `mise run test` | Run Rust, frontend, and isolated database tests |
| `mise run test:rust:unit` | Run hermetic Rust tests without PostgreSQL |
| `mise run test:rust:integration` | Run the PostgreSQL integration suite |
| `mise run build` | Build the application and frontend |
| `mise run test:e2e:install` | Install Chromium and its system dependencies (once) |
| `mise run test:e2e` | Run desktop/mobile browser tests against the test database |
| `mise run fmt` | Format and apply safe lint fixes |
| `mise run account:recover EMAIL [--enable]` | Issue a short-lived operator recovery link for an existing local account |
| `mise run mail:up` | Start optional development SMTP and the Mailpit inbox |

`mise run db:up` and `mise run db:test:up` can start services independently. Database tests use PostgreSQL on port 5433, separate from development on port 5432. SQLx creates a fresh database for each integration test, applies migrations, and removes successful test databases. The test server uses disposable storage; it is not a place to keep work. Browser tests run their own API on port 3001 and Vite on port 5174, creating a new database for each suite run and dropping it on graceful shutdown. They require a configured test-database name ending in `_test` and never clear the configured database.

The Rust workspace lives under `crates/`, with the `convex` application as its first crate. React, Vite, TypeScript, React Router, and the generated HTTP client types live in `web/`. `api/openapi.json` and `web/src/api/schema.d.ts` are generated artifacts: edit the Rust HTTP types and documented handlers, run `mise run generate`, and commit both files.

The frontend uses shadcn/ui's Nova style with Radix primitives, Tailwind CSS v4, Lucide icons, and a locally served Geist font. `web/components.json` configures the registry and `@/` aliases; owned component sources live in `web/src/components/ui/`. Add official components with `pnpm --dir web dlx shadcn@latest add @shadcn/<component>`. Theme colors and layout rules live in `web/src/styles.css`: use semantic tokens and component variants rather than global styles for buttons and inputs. Forms compose `FieldSet`, `FieldGroup`, and `Field`; the shared form wrapper preserves native validation, pending states, drafts, and API feedback. Compact form groups disable size containment so controls retain their natural width. The browser suite checks these controls on desktop and mobile alongside the real account and workspace flows.

CI uses the repository's `setup-repo` action and the same Mise tasks. Rust and Web checks, builds, and tests run in separate jobs; API contract verification and browser-to-API tests have dedicated jobs. Rust Build creates a Nextest archive once, and the Rust Unit Tests and Rust Integration Tests jobs download it and run their suites without recompiling. Both test jobs retain JUnit results; Rust checks run doctests separately because Nextest does not archive them. The aggregate `check`, `build`, and `test` tasks remain available locally. `test:rust` remains an alias for the hermetic suite and `test:db` for the integration suite. `mise run check` compiles and regenerates contracts before starting PostgreSQL. Current SQL queries use SQLx's runtime query API, so no query-macro metadata is needed. See [the implementation decisions](docs/backend-and-api.md#cp01-implementation-decisions) for conventions and tooling exceptions.

Keep hermetic Rust tests in source modules or database-free test targets. Register tests that require services explicitly in the crate's `Cargo.toml`, with a name beginning `integration_` and `required-features = ["integration"]`, and mark each such test ignored with its required service in the reason. The database suite is registered as `integration_database` in `crates/convex/tests/database.rs`. Nextest selects ordinary and integration binaries with complementary filters; integration tasks enable the feature, run ignored tests, and start the Compose test database. This keeps ordinary `cargo test` safe to run without services and includes all registered integration targets in CI.

## Roadmap

Start with the [canonical delivery checkpoints](docs/checkpoints.md) for the implementation path and completion criteria.

- [Issue foundation](docs/issue-foundation.md)
- [Issue data model](docs/issue-data-model.md)
- [Accounts and OIDC login](docs/authentication.md)
- [Backend and API contract](docs/backend-and-api.md)
- [Domain vocabulary](CONTEXT.md)

The stack is Rust, Axum, PostgreSQL, and SQLx on the backend, with utoipa-generated OpenAPI and generated TypeScript contract types for a React frontend. CP01 establishes the runnable application and contract pipeline. CP02 adds local accounts, invitations, sessions, recovery, and workspace/team settings; issue features follow in CP04.

Developer tooling uses Mise and Docker Compose almost exclusively: Mise manages toolchains and development tasks; Docker Compose runs local services and dependencies. See the [developer tooling policy](docs/backend-and-api.md#developer-tooling) for the planning requirements.
