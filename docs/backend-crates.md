# Backend crate boundaries

The backend remains one deployable application. Its crates separate HTTP handling,
authentication, domain rules, application workflows, and database machinery so a
production change has an obvious home.

## Where to look

| Crate | Responsibility | Starting points |
| --- | --- | --- |
| `convex` | Configuration, dependency wiring, server lifecycle, operator commands | `src/main.rs`, `src/runtime.rs`, `src/bin/` |
| `convex-api` | Axum routes, request/response DTOs, cookies, CSRF, OpenAPI, embedded frontend | `src/http.rs`, `src/accounts.rs`, `src/workspaces/mod.rs`, `src/issues/mod.rs` |
| `convex-auth` | Accounts, credentials, sessions, password recovery, login methods, OIDC, authentication policy, throttling and authentication email | `src/accounts/mod.rs`, `src/accounts/models.rs`, `src/repository.rs`, `src/security.rs`, `src/postgres/` |
| `convex-workspaces` | Workspaces, memberships, invitations, teams and workflows | `src/workspaces_model.rs`, `src/validation.rs`, `src/repository.rs`, `src/postgres/mod.rs` |
| `convex-issues` | Issue rules, persistence, activity and typed issue events | `src/model.rs`, `src/events.rs`, `src/repository.rs`, `src/postgres.rs` |
| `convex-app` | Cross-domain workflows and shared unit of work | `src/accounts/mod.rs`, `src/accounts/models.rs`, `src/account_workflows/mod.rs`, `src/workspace_workflows/mod.rs`, `src/issues/mod.rs`, `src/unit_of_work.rs` |
| `convex-db` | Connection setup, migrations, transaction storage, administration gate, readiness queries and disposable test databases | `src/postgres/mod.rs` |
| `convex-core` | Shared operation errors and name validation | `src/lib.rs`, `src/validation.rs` |

The API crate has no SQLx dependency. All Axum extraction, response serialization,
cookies, redirects, status codes and utoipa schemas live there. HTTP DTOs remain
distinct from domain models, with explicit conversions. Domain crates do not depend
on Axum or utoipa. `convex` selects implementations and starts the server; it contains
no handlers, domain rules, authentication internals or SQL.

Authentication services own their PostgreSQL workflows and provider protocol.
Application workflows own coordination between authentication and workspace/issue
repositories. OIDC uses an application-supplied invitation gateway, borrowing the
existing transaction, so authentication does not depend on the workspace domain.

## Repositories and features

Repository traits describe operations and persistence guarantees for accounts,
sessions, installation state, workspaces, memberships, teams, workflows,
invitations and issues. Value objects and request/result projections do not need
independent repositories. There is no generic CRUD abstraction.

`convex-auth`, `convex-workspaces`, `convex-issues` and `convex-app` have no default
features. `postgres` enables SQLx implementations; `mock` enables Mockall
implementations. Both may be enabled together. `convex-db/postgres` supplies database
mechanics, and `convex-db/testing` additionally supplies owned disposable databases.
`convex-core/postgres` maps database failures into operation errors. The executable
explicitly selects production features; application and API tests enable mocks through dev-dependencies.
No-adapter and mock-only builds do not depend on SQLx.

The PostgreSQL adapters implement repository traits directly for
`PostgresTransaction`. Each domain owns its SQL; the database crate does not depend
on domain crates. `convex-app` composes these implementations into its unit of work.
Repository getters borrow the carrier mutably, one at a time, so every query uses
the same connection without a mutex or a hidden pool.

The HTTP adapter accepts prepared application services. Authentication, login-method
and OIDC services also expose transport-independent, mockable interfaces. Their
PostgreSQL implementations live entirely in `convex-auth`.

## Following a membership removal

1. The API parses the workspace path, credentials and membership command.
2. The application validates the command and begins an administration unit of work.
3. The database adapter acquires the installation gate before account/workspace locks.
4. The account repository authenticates the actor; the membership repository locks
   the workspace and checks the actor's role and the target membership.
5. Workspace rules protect administration privileges and the final active owner.
6. The issue repository clears assignments and writes activity; the membership
   repository removes the member; the session repository revokes their sessions.
7. The carrier commits once. Only then does the application return success.

Any failure rolls back the shared transaction. Dropping an unfinished PostgreSQL
transaction also rolls it back, including cancellation. Invitation acceptance,
initial setup and account disablement use the same carrier across their domains.
Invitation email delivery occurs after commit; a delivery failure still allows the
administrator to share the committed invitation link.

Issue operations use `begin()` and scoped workspace/issue locks rather than the
installation gate. Administration holds exclusive workspace locks, preventing
assignment changes during membership removal or account disablement. An issue edit
locks and loads the current version, applies domain rules, validates references,
saves the issue and records activity before committing. No-op edits and version
conflicts do not write or record events.

OIDC consumes a flow before remote exchange, releases its transaction while
contacting the provider, then rechecks policy, session, identity and invitation
state in a second transaction. Failed exchanges cannot replay a consumed flow.
Authentication policy activation owns a dedicated lease connection per replica;
protected requests recheck that lease without contacting the provider.

## Events

Typed issue events describe creation, updates, archival and restoration. Before
and after snapshots feed activity records inside the mutation's transaction,
preserving the stored shape and historical names. Issue application results expose
events only after commit; HTTP responses retain their existing shape.

There is no dispatcher or external delivery. Reliable notifications would require
an outbox written in the same transaction. Required cross-domain consequences,
such as assignment cleanup during membership removal, are explicit transactional
operations.

## Verification

- `mise run check:rust` checks formatting, workspace linting, adapter feature
  configurations, the independent API build and doctests.
- `mise run test:rust:unit` runs HTTP/application/domain behavior without a database.
- `mise run test:rust:integration` checks real queries, constraints, shared
  transactions, rollback and concurrent locking against disposable PostgreSQL.
- `mise run generate:check` verifies the existing OpenAPI and TypeScript artifacts.
- `mise run check:bundle` verifies the embedded frontend with all features enabled.

Unit tests beside the `convex-app` workflow modules exercise setup, account
administration, workspace permissions, invitations, issue outcomes and event
snapshots directly. Mockall repositories assert transaction ordering and catch
unexpected writes on rejected operations. These tests run with ordinary crate
tests, including `cargo test -p convex-app --no-default-features`, without SQLx or
a database. Lifecycle tests also check that commit errors prevent success and
rollback errors preserve the original operation error.

The issue and workspace endpoint tests in `convex-api` run the production router,
middleware and application workflows with Mockall repositories and policy checks.
They need no database pool. They cover mutation ordering, rollback, commit failures,
DTO conversion and session cookie behavior. PostgreSQL integration tests establish
actual atomicity, constraints, policy leases, provider validation and lock behavior.
