# Proposed issue management foundation

Build Convex, an open-source replacement for Linear, beginning with a usable issue workflow. The first milestone should let a small team capture work, assign it, discuss it, and track it through completion.

## Agreed direction

Define the scope and data model before implementation. Use Rust with Axum, PostgreSQL, and SQLx for the backend, and React with TypeScript for the frontend. Generate OpenAPI specifications with utoipa and utoipa-axum, derive TypeScript contract types from the specification, and check generated artifacts in CI from the first implementation checkpoint. The [backend and API plan](backend-and-api.md) describes this agreed workflow and the proposed HTTP conventions.

The first release serves other teams installing Convex themselves, with a polished core of issues, labels, comments, and search. Include built-in accounts and configurable OpenID Connect (OIDC) login in that release. [The authentication plan](authentication.md) distinguishes these agreed capabilities from proposed behavior.

Use Mise and Docker Compose almost exclusively for developer tooling: Mise manages toolchains and development tasks, while Compose runs local services and dependencies. The [developer tooling policy](backend-and-api.md#developer-tooling) defines this agreed convention.

Convex is the working product name. Naming can be revisited later; it is not a blocker for scope, modeling, or implementation.

The proposed [relational data model](issue-data-model.md) expands the domain relationships and invariants below. [CONTEXT.md](../CONTEXT.md) defines the vocabulary.

## First milestone

- Create, read, edit, archive, and restore issues.
- Give every issue a stable human-readable identifier, title, formatted description stored as Markdown, status, priority, and optional assignee.
- Support workspace labels and team-specific workflow statuses.
- Provide searchable issue lists with filters for team, status, priority, assignee, and label, plus sorting and grouping.
- Show issue details with editable properties, comments, and an activity history.
- Support keyboard navigation, quick issue creation, and direct links to issues.
- Persist changes and enforce workspace membership on every read and write.
- Generate and publish the OpenAPI contract alongside the application.
- Support local accounts, OIDC login, first-admin setup, and a way to invite teammates.
- Provide release images, installation and upgrade instructions, and tested backup/restore guidance.

One workspace and one team can be the initial onboarding path. Keep their ownership explicit so adding more does not require changing what an issue means.

Start implementation with a narrower vertical slice: authenticated members can create an issue, find it in a list, open its details, and change its title, description, status, priority, or assignee. Labels, discussion, and navigation complete the broader milestone afterward.

## Domain relationships

| Concept | Relationships |
| --- | --- |
| Workspace | Contains teams, memberships, and labels |
| Membership | Connects a person to a workspace |
| Team | Belongs to one workspace; owns issues and statuses |
| Issue | Belongs to one team; has one status and zero or one assignee |
| Status | Belongs to one team; has one workflow category |
| Label | Belongs to one workspace; can classify many issues |
| Comment | Belongs to one issue; has an author |
| Activity entry | Belongs to one issue; records an actor, change, and time |

## Proposed invariants

- An issue's status must belong to its team.
- An assignee must have active membership in the issue's workspace.
- Labels applied to an issue must belong to its workspace.
- Issue numbers are unique within their team and are never reused, including after archival. Team keys are immutable in the first milestone; moving issues between teams is deferred.
- Status names can change without changing the meaning of their workflow categories.
- A status in use cannot be removed without choosing a replacement for affected issues.
- Removing a member preserves their historical authorship and clears their active assignments.
- Archiving preserves comments, activity, and links; archived issues are excluded from ordinary lists.
- A change to an issue and its corresponding activity entry succeed together.
- Concurrent edits to different properties preserve both changes; conflicting edits to the same property need an explicit policy.

## Delivery checkpoints

The canonical sequence, dependencies, statuses, and completion criteria live in [Delivery checkpoints](checkpoints.md). CP01 through CP08 deliver the first self-hosted alpha. This document defines its feature scope; the roadmap defines how that scope is delivered and verified.

## Later capabilities

Projects, cycles, issue relationships and dependencies, sub-issues, saved views, notifications, integrations, attachments, reporting, and automation can follow the issue foundation. Each needs its own behavior and acceptance criteria when introduced.

## Decisions still open

- Domain-route and frontend interaction choices beyond the CP01 Vite/React Router and OpenAPI TypeScript foundation.
- Self-hosting packaging and operational details.
- Account linking, invitation, recovery, and OIDC configuration policies.
- Open-source license.
- Description storage, default issue status, team-key changes, and archived-issue editability. Current model choices remain proposals.

The working name, language choices, Axum/PostgreSQL/SQLx backend, utoipa generation, scope-first approach, self-hosted audience, core feature focus, and support for local and OIDC login are agreed. Detailed behavior and relational design remain proposals. Projects, cycles, sub-issues, dependencies, and board views follow the first release.
