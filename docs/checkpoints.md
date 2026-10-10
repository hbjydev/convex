# Convex delivery checkpoints

This is the canonical delivery roadmap for Convex. The first release is a self-hosted issue tracker that other teams can install and maintain, with a polished core of issues, labels, comments, and search, plus built-in accounts and OIDC login.

CP01 through CP08 define the path to the first self-hosted alpha. CP09 onward describes the foreseeable expansion toward a fuller replacement for Linear; their order and detailed scope should be revisited after teams use the alpha. No delivery dates are assigned.

## Current position

CP01, CP02, and CP04 are complete. Convex now has the runnable application and generated API contract, local accounts and workspace onboarding, and a persisted issue workflow with creation, assignment, editing, conflict resolution, archival, and restoration. Completion checks passed locally; CP03 and CP05 onward remain planned.

- Backend: Rust, Axum, PostgreSQL, and SQLx.
- Contract: utoipa and utoipa-axum generate OpenAPI alongside route registration; TypeScript contract types are generated from that specification and checked in CI.
- Frontend: React and TypeScript.
- Developer tooling: Mise and Docker Compose almost exclusively, following the [developer tooling policy](backend-and-api.md#developer-tooling).
- Authentication: built-in accounts and configurable OIDC login.
- First audience: other teams installing Convex themselves.

## How to use this roadmap

Use checkpoint IDs in implementation tasks and progress updates. Keep the IDs stable as details change. Update a checkpoint's status to in progress when work starts and complete only when its completion criteria have been verified. Record verification evidence when marking it complete.

This document owns sequencing, dependencies, and release completion. The linked design documents own detailed behavior. Where they still describe proposals, settle the relevant choices before implementing that behavior; those proposals are not made final merely by appearing in a checkpoint.

Build each checkpoint across the backend and frontend where applicable. Include its relevant tests and OpenAPI changes in the same work. Permissions, transactions, concurrency, and activity recording accompany the operations that need them.

## First self-hosted alpha

| ID | Checkpoint | Depends on | Status |
| --- | --- | --- | --- |
| CP01 | Application foundation and API contract | Agreed stack | Complete |
| CP02 | Local accounts and workspace onboarding | CP01 | Complete |
| CP03 | OIDC login and account linking | CP02 | Planned |
| CP04 | Complete issue workflow | CP02 | Complete |
| CP05 | Labels and finding work | CP04 | Planned |
| CP06 | Discussion and activity history | CP04 | Planned |
| CP07 | Daily-use polish | CP03, CP05, CP06 | Planned |
| CP08 | Self-hosted alpha release | CP07 | Planned |

CP03 and CP04 can progress once CP02 is complete. CP05 and CP06 both build on CP04. All eight checkpoints are required for the alpha.

## CP01 Application foundation and API contract

Deliver a runnable Rust application and React frontend with a working development environment, PostgreSQL migrations, and the contract-generation pipeline.

Included work:

- Establish repository structure, dependency versions, configuration, and logging. Pin toolchains with Mise, expose standard development/check/generation workflows as Mise tasks, and run local services through Docker Compose.
- Connect Axum and SQLx to PostgreSQL and establish migration and test-database workflows.
- Establish public request/response types, a consistent error format, and documented routing.
- Generate OpenAPI through utoipa and utoipa-axum, export the specification without running the application, and generate TypeScript contract types.
- Add a shared frontend HTTP client and a small end-to-end request using the generated types.
- Add CI for Rust/frontend checks and generated-artifact consistency, with SQLx metadata supporting compilation without a live database where query macros require it.

Complete when a fresh checkout can run using the documented Mise tasks and Docker Compose setup, the frontend makes a documented request to the backend, migrations work on an empty database, and CI uses the same task entry points to regenerate and verify the contract artifacts. Document any necessary tooling exceptions and their reasons.

Resolve here: frontend build tooling and routing, the TypeScript generator, HTTP conventions, and the initial application packaging structure. See [Backend and API contract](backend-and-api.md).

Verification recorded on 2026-10-08:

- `mise run check` passed Rust formatting/Clippy, Biome, TypeScript, and generated-artifact consistency checks.
- `mise run test` passed one database-free router/contract test, two frontend HTTP-client tests, and two isolated PostgreSQL integration tests. Live success/error bodies were validated against generated schemas.
- `mise run db:migrate` passed twice against the development database; isolated tests verified migration on an empty database and repeat application. Connections explicitly pin the migration ledger's search path to avoid PostgreSQL role/schema name collisions.
- `mise run build` passed. A clean source copy without `node_modules`, compiled artifacts, or `.env` passed `mise install`, `mise run setup`, `mise run check`, and `mise run build`. `mise run dev` then served the frontend, proxied a healthy API request, and served a specification equal to the exported artifact; Ctrl-C shut down the API.
- `mise run test:e2e` passed four desktop/mobile Chromium checks for a real browser-to-API request and recovery after network failure. Desktop and mobile screenshots were inspected; neither viewport overflowed.
- `mise run generate:check` passed with both Compose databases stopped, confirming the contract pipeline needs no live database.
- CI uses the existing `setup-repo` action and those Mise task entry points. The workflow passed Actionlint. A hosted GitHub Actions run has not yet been observed in this environment.

The [README](../README.md#development) documents the normal setup, and the [CP01 decisions](backend-and-api.md#cp01-implementation-decisions) record packaging, routing, generator compatibility, HTTP conventions, SQLx metadata policy, and tooling exceptions.

## CP02 Local accounts and workspace onboarding

Deliver the path from a fresh installation to two people working in a shared workspace through built-in accounts.

Included work:

- Installer-authorized, one-time first-admin setup and creation of the initial workspace and team.
- Local account creation through the agreed onboarding policy, login, logout, session management, and password changes/recovery.
- Workspace memberships, roles, teammate invitations, and membership removal.
- Team creation and initial workflow statuses, with the configuration needed for issue creation.
- Account disablement and a documented operator-recovery path.
- Frontend onboarding, login, invitation acceptance, and essential workspace/team settings.

Complete when a fresh instance can create its first admin, onboard a second person, authenticate both, and enforce workspace roles. Verify invitation single-use behavior, session revocation, workspace isolation, and membership removal. Demonstrate the chosen onboarding and recovery paths with SMTP unavailable as well as configured, where applicable.

Resolve here: signup/invitation policy, roles, email handling, session/CSRF policy, SMTP behavior, recovery, and workflow defaults. See [Accounts and OIDC login](authentication.md) and [Issue data model](issue-data-model.md).

Verification recorded on 2026-10-08:

- `mise run check` passed Rust formatting/Clippy, Biome, TypeScript, and database-free generated-artifact consistency checks. `mise run build` passed for both the application and frontend.
- `mise run test:rust:unit` passed two hermetic tests, and `mise run test:web` passed two HTTP-client tests. `mise run test:rust:integration` passed eight isolated PostgreSQL tests; exercised success and error responses were validated against generated schemas.
- Database tests verified installer authority and concurrent one-time setup; concurrent single-use invitation acceptance and password reset; password/session revocation; expiration and throttling; role restrictions and inviter authority; membership reactivation without duplicate rows; last-owner protection; account disablement; and cross-workspace isolation.
- SMTP-unavailable tests verified direct invitation sharing, uniform recovery responses, and operator-issued recovery tokens. A configured loopback SMTP fixture received invitation and recovery messages, and its reset link changed the password and revoked sessions. `mise run mail:up` also started the pinned development Mailpit service successfully. Production SMTP/STARTTLS delivery has not been exercised against an external provider.
- `mise run test:e2e` passed eight desktop/mobile Chromium scenarios. Two people completed onboarding and local authentication; invitation replay failed; password change and membership removal revoked access; the real `recover-account --enable` binary re-enabled a disabled local account, issued a reset link, and completed browser password recovery without restoring removed membership. Browser tests use a fresh database and fixed test origin/configuration, independent of local `.env` overrides.
- Browser checks verified API/network recovery, retained login drafts after a failed request, and no overflow for valid 100-character unbroken workspace/team/display names. Desktop and mobile onboarding, invitation, login/recovery, account, and workspace-settings screenshots were inspected. The UI reviewer scored its long-name and network-error fixes resolved; the documentation check confirmed the existing visual system and CP02 documentation.

The [CP02 implementation decisions](authentication.md#cp02-implementation-decisions) record onboarding, roles, email normalization, sessions/CSRF, SMTP, recovery, and workflow defaults. The [installation and operator-recovery instructions](authentication.md#first-installation-and-operator-recovery) document both mail-enabled and mail-unavailable operation.

## CP03 OIDC login and account linking

Deliver configurable login through an existing identity provider, using the same accounts, sessions, and workspace permissions as local login.

Included work:

- Provider configuration, discovery, authorization-code login, callback validation, and frontend login controls.
- Invitation acceptance through OIDC and correct resolution of existing external identities.
- Explicit linking/unlinking of login methods, with proof of control and protection against removing the last usable method.
- Clear behavior for logout, disabled accounts/providers, missing provider configuration, and provider outages.
- Operator documentation and a reproducible development/test identity-provider setup through Docker Compose and Mise tasks.

Complete when local and OIDC users can use the same workspace, an account can use both methods after linking, and matching emails do not merge accounts or grant membership. Verify invalid/replayed callbacks, linking conflicts, revoked sessions, and local operator recovery during a provider outage.

Resolve here: installation-level versus workspace-level providers, session behavior after provider changes, and whether operators can enforce OIDC for ordinary users. See [Accounts and OIDC login](authentication.md).

## CP04 Complete issue workflow

Deliver the first usable create → list → detail → edit flow for authenticated workspace members.

Included work:

- Persisted issues with title, description, team, status, priority, optional assignee, and creator.
- Internal IDs, human-readable identifiers, and stable direct links.
- Issue creation, list/detail queries, property updates, archival, and restoration.
- Server-side validation, scoped authorization, version-based conflict handling, and transactional activity recording.
- Frontend creation, issue list, detail/editor, property controls, and clear update-conflict behavior.

Complete when two people can create, find, open, assign, update, archive, and restore an issue. Verify simultaneous number allocation, workspace isolation, invalid references, update conflicts, and rollback of both a mutation and its activity. A reload preserves the final state and issue links still resolve.

Resolve here: description storage/editor behavior, default status, team-key policy, archived-issue behavior, and the exact patch/conflict contract. See [Issue foundation](issue-foundation.md) and [Issue data model](issue-data-model.md).

Verification recorded on 2026-10-10:

- `mise run check`, `mise run build`, and `mise run check:bundle` passed, including generated OpenAPI/TypeScript consistency and embedded frontend routing checks.
- `mise run test:rust:unit` passed three tests; `mise run test:web` passed three tests. `mise run test:rust:integration` passed 13 isolated PostgreSQL tests, with exercised HTTP responses validated against the generated schemas.
- Database tests verified simultaneous number allocation, workspace isolation, team-scoped lists, invalid scoped references, nullable patch semantics, stale-version conflicts, archive/restore, stable identifiers after team renaming, and transactional rollback of creation, editing, archival, activity, and membership removal. Concurrent assignment/removal completed without deadlock and left no assignment to the removed member.
- `mise run test:e2e` passed all 44 desktop/mobile Chromium scenarios. Two authenticated people exercised issue creation, assignment, edits, conflict reconciliation, archival, and restoration; reloads preserved state and issue-key links resolved. Network failures retained drafts. The final conflict-focus change additionally passed both focused issue scenarios.
- The supplied UI reference informed the compact status-grouped list and centered creation dialog. Browser checks covered property menus, Create more, draft reopening, group collapse, and keyboard focus. Populated lists, creation dialogs, and conflict/error states were inspected in desktop/mobile and light/dark themes.
- The sidebar follow-up adds collapsible Home/Issues links per team and routes under `/<workspace-slug>/team/<lowercase-team-key>`: Home, `/all`, `/archive`, and `/new`. Desktop/mobile checks verified switching two teams, team creation defaults, reloads, and archive scope. `mise run check:web` and `mise run build:web` passed; final team-navigation captures were inspected in both themes.
- Issue detail URLs now use `/<workspace-slug>/issue/<issue-key>/<issue-title-slug>`. Both focused desktop/mobile issue scenarios passed with key-only and stale-title redirects, URL updates after title changes without losing editor/conflict drafts, back navigation, owning-team sidebar selection, Unicode/symbol-only titles, and rejection of unsupported routes. `mise run test:web` passed four tests, including slug generation; `mise run build:web` passed.
- The final sidebar removes duplicate workspace and settings links in favor of the top dropdown. `mise run check:web` and all 36 desktop/mobile issue and UI browser scenarios passed after this change; updated sidebar captures were inspected.
- The initial description follow-up opened issue details with rendered Markdown and a properties column, using the standalone shadcn typeset stylesheet. Editing includes a source preview and save/cancel protection for dirty drafts. `mise run check:web`, `mise run build:web`, all six frontend unit tests, and both desktop/mobile issue scenarios passed. Checks covered Markdown/GFM rendering, raw HTML and unsafe-link filtering, preview/source round-tripping, conflicts, and archive/restore; reading and preview captures were inspected in both themes.

- The in-place editing follow-up replaces the source/preview mode with a borderless Tiptap/ProseMirror description and editable property menus. Title/description changes save after a pause or on blur; property selections save immediately. Serialized writes preserve edits made during saves, failures retain drafts, conflicts require review, and navigation guards unresolved edits. `mise run check:web` and `mise run build:web` passed, along with all 14 frontend unit tests and both desktop/mobile issue workflows, covering Markdown structure, task checkbox persistence, in-flight edits, retry, conflict focus, reloads, and archive/restore. Desktop/mobile layouts were inspected in both themes.

The [CP04 implementation decisions](issue-data-model.md#cp04-implementation-decisions) record description limits, defaults, retention, authorization and locking, patch semantics, and activity snapshots. Labels/search and visible discussion/history remain CP05 and CP06 work.

## CP05 Labels and finding work

Deliver a navigable backlog that a team can search and organize as it grows.

Included work:

- Workspace label creation, management, and issue assignment/removal, with recorded activity.
- Search over issue titles, descriptions, and human identifiers.
- Filters for team, status, priority, assignee, and labels; sorting and grouping.
- Pagination with deterministic ordering, and URL state for repeatable searches/filters.
- Loading, empty, and no-result states, with permission-aware queries and query-plan-driven indexing.

Complete when a member can reliably find work by identifier, text, or combined filters, reload a filtered URL, and navigate multiple pages. Verify workspace-scoped labels, search isolation, filter semantics, and representative query performance using a recorded dataset and target. Define how updates affect a paginated list.

Resolve here: search matching/ranking, multiple-label filter semantics, pagination, grouping, and label lifecycle. See [Issue data model](issue-data-model.md) and [Backend and API contract](backend-and-api.md).

## CP06 Discussion and activity history

Deliver the context needed to understand an issue and collaborate on it.

Included work:

- Comment creation, editing, deletion, and authorship.
- An issue timeline showing comments and the activity recorded since CP04, including label changes from CP05 when available.
- Clear edit/deletion indicators and understandable historical property values.
- Paginated history and preservation of historical authorship when members leave.
- Appropriate permissions and concurrent comment-edit handling.

Complete when two members can discuss an issue and read how its properties changed. Verify author permissions, edits/deletions, historical records after member removal, and deterministic timeline ordering/pagination. Archived-issue behavior follows the policy settled in CP04.

Resolve here: comment formatting, deletion/retention rules, timeline presentation, and attribution snapshots. See [Issue data model](issue-data-model.md).

## CP07 Daily-use polish

Deliver a coherent experience that is comfortable to use for everyday issue work.

Included work:

- Keyboard navigation, quick issue creation, focus management, and accessible controls.
- Consistent navigation between workspace/team lists and issue details.
- Responsive layouts and readable descriptions, comments, and property controls.
- Loading, empty, validation, permission, network-error, and conflict states across the core flows.
- Predictable save behavior and feedback that protects edits when a request fails.
- A defined strategy for refreshing other users' changes, with its behavior tested across two sessions.

Complete when the full local-account and OIDC workflows work with keyboard navigation and two concurrent users. Record accessibility and representative browser/screen-size checks, fix material usability defects, and verify that failures do not silently discard edits or falsely report success.

Resolve here: interaction conventions, update-refresh strategy, responsive scope, and supported browsers. Real-time transport is an implementation choice; the required outcome is understandable and reliable collaboration.

## CP08 Self-hosted alpha release

Deliver a version another team can install, operate, upgrade, and recover using the published instructions.

Included work:

- Versioned application images and a documented deployment setup with PostgreSQL, configuration, persistent storage, and HTTPS expectations.
- First-run onboarding, OIDC configuration, optional SMTP, and operator-recovery instructions.
- A defined migration/upgrade policy, failure behavior, and backup/restore procedure.
- Health/readiness checks, operational logs, and troubleshooting guidance.
- Release artifacts including the matching OpenAPI specification, release notes, and documented limitations.
- An explicit open-source license and contribution/setup documentation before public release.

Complete when a clean environment can install the release, onboard two users, complete the issue workflow, and preserve data through restart. Test an upgrade from a previous test build with representative data, then restore a backup into a fresh environment and verify accounts, memberships, issues, labels, comments, and history. Another person must be able to follow the installation instructions without undocumented developer intervention.

The alpha is complete only when CP01 through CP08 meet their completion criteria. Claims about supported identity providers, deployment configurations, and browsers must match recorded verification.

## Later checkpoints

These are forecast checkpoints, not commitments to dates or a fixed order. Define their detailed model, interactions, dependencies, and completion criteria when bringing them into active scope. Use feedback from installed teams to choose which comes next.

| ID | Checkpoint | Foreseeable scope | Completion target |
| --- | --- | --- | --- |
| CP09 | Issue depth and views | Sub-issues, dependencies/relations, board views, saved views, and issue transfers. Consider estimates and due dates here. | A team can express related work and choose useful views without losing identifiers or workflow context. |
| CP10 | Projects | Project ownership, scope, progress, and milestones; connecting issues to larger outcomes. | A team can plan and follow a project through completion using its issues. |
| CP11 | Cycles and planning | Team planning periods, capacity, committed work, and rollover behavior. | A team can plan and review a cycle with clear treatment of unfinished work. |
| CP12 | Notifications and richer collaboration | Subscriptions, mentions, inbox/email notifications, attachments, and reactions. | Members receive actionable updates with defined delivery and access behavior, and can share supporting material. |
| CP13 | Integrations and data portability | Imports/exports, GitHub/GitLab links, webhooks, and integration credentials. | A team can bring work in, connect development activity, and take its data out. |
| CP14 | Reporting and automation | Workflow metrics, dashboards, recurring work, rules, and templates. | A team can inspect progress and automate repeated actions with attributable, reliable results. |
| CP15 | Administration at larger scale | Private teams, finer permissions, additional identity-provider policies, audit tooling, and provisioning. | Larger installations can govern access and operate the product with documented behavior. |

Offline-first behavior, native mobile/desktop clients, and a hosted offering remain separate possible directions. They need an explicit product decision before becoming checkpoints.

## Supporting plans

- [Issue foundation](issue-foundation.md): first-release feature scope and proposed invariants.
- [Issue data model](issue-data-model.md): relational entities, mutations, and acceptance scenarios.
- [Accounts and OIDC login](authentication.md): identity, sessions, invitations, and recovery.
- [Backend and API contract](backend-and-api.md): application structure and contract generation.
- [Domain vocabulary](../CONTEXT.md): canonical product terms.
