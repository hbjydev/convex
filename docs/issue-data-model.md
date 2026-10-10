# Proposed relational issue data model

This model supports Convex's first issue-management milestone using Rust, Axum, SQLx, and PostgreSQL. It describes persisted entities and their rules before writing migrations. UUIDs are internal references; human-readable issue identifiers are a separate concept. The HTTP contract is defined separately in the [backend and API plan](backend-and-api.md).

CP02 now persists accounts, workspace memberships, teams, and their initial workflow statuses. Its settled policies are recorded in [CP02 authentication decisions](authentication.md#cp02-implementation-decisions). CP04 now implements issues and transactional activity; its settled policies are recorded in [CP04 implementation decisions](#cp04-implementation-decisions). Labels and comments below remain proposals for later checkpoints.

## Shared conventions

- Each entity has an immutable UUID primary key unless a composite key is specified below.
- Timestamps represent UTC instants.
- Foreign keys preserve historical records. Routine member removal and issue archival are lifecycle changes, rather than cascading physical deletes.
- Workspace ownership is enforced in relational constraints and application authorization. Knowing a UUID never grants access.
- Required strings have explicit length limits at implementation time. Issue titles and names cannot be blank after trimming.

## Identity and ownership

| Table | Fields | Rules |
| --- | --- | --- |
| `users` | `id`, `display_name`, optional `avatar_url`, `is_instance_admin`, optional `disabled_at`, `created_at`, `updated_at` | A durable user account shared across login methods. Instance administration is separate from workspace membership. |
| `workspaces` | `id`, `name`, `created_at`, `updated_at` | Owns teams, memberships, and labels. |
| `workspace_memberships` | `id`, `workspace_id`, `user_id`, `role`, `joined_at`, optional `removed_at` | Unique `(workspace_id, user_id)`; proposed roles are owner, admin, and member. Reactivating a membership reuses the same row. |
| `teams` | `id`, `workspace_id`, `name`, `key`, `next_issue_number`, `created_at`, `updated_at` | Unique `(workspace_id, key)`; uppercase keys such as ENG are immutable initially. |

The first milestone uses workspace-wide access: active members can view and edit issues across its teams. Team membership and private teams are later capabilities. Owners and admins manage membership and team configuration; members manage issues and their own comments. Role changes or removals must preserve at least one active workspace owner.

Memberships, rather than users alone, identify assignees and actors. This makes their relationship to a specific workspace explicit. Removed memberships remain available to resolve historical authorship.

Local credentials, OIDC identities, sessions, and invitation/recovery tokens are separate from users and workspace memberships; see the proposed [authentication model](authentication.md). Successfully authenticating does not create workspace access. Disabling an account prevents all its login methods from being used and revokes its sessions.

## Workflow and classification

| Table | Fields | Rules |
| --- | --- | --- |
| `workflow_statuses` | `id`, `workspace_id`, `team_id`, `name`, `category`, `position`, `created_at`, `updated_at` | Belongs to its team's workspace; unique name within a team; category is backlog, unstarted, started, completed, or canceled. |
| `labels` | `id`, `workspace_id`, `name`, `color`, `created_at`, `updated_at` | Unique normalized name within a workspace. |
| `issue_labels` | `workspace_id`, `issue_id`, `label_id` | Composite primary key `(issue_id, label_id)`; issue and label belong to the same workspace. |

Suggested initial statuses are Backlog, Todo, In Progress, Done, and Canceled. Names and ordering are editable. Deleting a status that issues use requires a replacement status in the same team. Every team retains at least one status and one unstarted status; issue creation defaults to its first unstarted status by position.

Priority is a fixed issue value: none, urgent, high, medium, or low. It is independent of workflow status. Priority sorting uses explicit urgency order, rather than alphabetic order.

## Issues

| Field | Meaning |
| --- | --- |
| `id` | Immutable internal UUID; canonical identity for links and references. |
| `workspace_id` | Owning workspace, constrained to match the team. |
| `team_id` | Owning team; unchanged during the first milestone. |
| `number` | Positive sequence number allocated within the team. |
| `title` | Required plain text. |
| `description` | Markdown text, including an empty description. An editor can render and edit this without making its internal document format the storage contract. |
| `status_id` | Required workflow status from the issue's team. |
| `priority` | One of the fixed priority values; defaults to none. |
| `assignee_membership_id` | Optional active membership in the same workspace. |
| `creator_membership_id` | Membership that originally created the issue; retained after departure. |
| `version` | Positive integer incremented on each issue mutation for concurrency control. |
| `created_at`, `updated_at` | Creation and latest issue mutation times. |
| `archived_at`, `archived_by_membership_id` | Both absent for an active issue, both present for an archived issue. |

The table has unique `(team_id, number)`. The human identifier is formed from the immutable team key and number, such as ENG-42, and is unique within a workspace. A link carries workspace context and resolves to the internal issue UUID. Team name changes do not change identifiers.

Allocate a number by atomically incrementing the team's counter in the same transaction as issue creation. Never calculate it as `MAX(number) + 1`. Gaps are acceptable; duplicates and reuse are not. Counter increments roll back if the creation transaction rolls back.

Persisted `workspace_id` on child records is deliberately redundant. Composite foreign keys must keep that ownership consistent: `(workspace_id, team_id)` references a team; `(workspace_id, team_id, status_id)` references a status; `(workspace_id, membership_id)` references a membership. Referenced tables need matching unique keys. Constraints check ownership; the application additionally checks active membership and permission.

## Discussion and history

| Table | Fields | Rules |
| --- | --- | --- |
| `comments` | `id`, `workspace_id`, `issue_id`, `author_membership_id`, `body`, `version`, `created_at`, `updated_at`, optional `deleted_at` | Markdown body; author must be an active member when posting. Deletion leaves a tombstone, with the body cleared. |
| `issue_activity` | `id`, `workspace_id`, `issue_id`, `actor_membership_id`, `kind`, `payload`, `created_at` | Append-only activity written in the same transaction as the recorded change. Actor must be active at the time of the action. |

Initial activity kinds cover issue creation, property updates, label addition/removal, and issue archival/restoration. Payloads have a validated shape for each kind, including old and new values for property changes. Snapshot display values alongside referenced IDs so a later rename does not rewrite the meaning of old activity.

Comments are displayed alongside activity, but their bodies are not duplicated into the immutable activity table. Comment edits are marked with `updated_at`; a deleted comment shows its tombstone. Notification delivery, full comment revision history, and automation actors are separate later decisions.

## Mutation behavior

Issue updates accept a property patch and an expected issue version. A transaction verifies workspace membership, loads the issue, checks its version, validates changed references, updates the requested properties, increments the version, and appends activity. A version mismatch returns a conflict with the current issue so the client can reconcile; no changes are applied.

After a conflict, the client can reapply a patch automatically only when the patched properties are unchanged from its original snapshot. If a property changed, show the conflict for explicit resolution. Do not replace the whole issue from stale client state. Comment edits use their own version, so discussion changes do not conflict with property edits.

Label changes participate in the issue version and activity transaction. Posting a comment changes neither the issue version nor its property history. Archiving and restoring do change the issue version. Archived issues remain readable; edits and new comments require restoration first.

Removing a membership and clearing its assignments happen in one transaction, with assignment changes recorded against the acting admin. Operations that assign work coordinate with membership removal so a removed member cannot receive a new assignment during the race.

## Query shapes

- List active issues in a workspace and optional team, filtering by status, priority, assignee, and labels. Use stable pagination with the issue UUID as a final sort tie-breaker.
- Resolve an issue by its UUID or workspace-scoped human identifier.
- Fetch comments and activity separately with pagination and deterministic ordering by creation time and UUID.
- Find assignable members by workspace and active membership.
- Search titles, descriptions, and issue identifiers. Start with PostgreSQL full-text search for prose and explicit identifier lookup; an external search service is not required for the initial design.

Likely initial indexes cover active issues by workspace/team and creation time, filtered status and assignee queries, comments/activity by issue and time, and labels in both directions. Exact PostgreSQL indexes follow measured query plans, rather than indexing every property combination.

## Acceptance scenarios

1. Two simultaneous issue creations in the same team receive distinct numbers.
2. Renaming a team preserves ENG-42 and all existing issue links.
3. A member of workspace A cannot read an issue, post a comment, or assign a person in workspace B, even with valid foreign UUIDs.
4. Assigning an issue to a removed membership fails; removing its current assignee clears the assignment and preserves their older comments.
5. Changing a status and writing its activity either both succeed or both roll back.
6. Two edits from the same original issue version produce one success and one conflict; neither silently overwrites the other.
7. Archiving an issue removes it from ordinary lists but preserves its details, comments, and identifier; restoration makes it editable again.
8. Renaming a status preserves its category and readable historical changes.

## Deferred extensions

Projects and cycles should be separate entities referenced by issues when their behavior is defined. Dependencies should use typed issue relationships; sub-issues need explicit hierarchy rules. Cross-team issue transfers require a decision about identifier aliases and status mapping. Attachments, reactions, due dates, estimates, subscribers, notifications, saved views, board ordering, and private teams are intentionally absent from the first schema proposal.

CP04 settles issue retention, description storage, and mutation behavior below. CP02 settles local authentication, workspace permissions, and workflow defaults; OIDC behavior remains CP03 work. Unimplemented collaboration models remain proposals.

## CP04 implementation decisions

CP04 is implemented and verified. Issues store Markdown descriptions (up to 50,000 characters) and trimmed plain-text titles (1–240 characters). The detail view edits descriptions in place through Tiptap/ProseMirror using shadcn typeset styles, supporting headings, formatting, task lists, tables, images, links, and code. Markdown is parsed into the supported document schema, unsafe image/link protocols are filtered, and HTML is never inserted directly into the page. Opening an issue does not rewrite its description. Editing serializes the supported document back to Markdown. Title and description edits save after an 800ms pause or on blur; property selections save immediately. A single write queue retains edits made during a pending request. Failed requests preserve drafts with retry/discard actions, conflicts require explicit review, and navigation flushes edits or guards unresolved drafts. Archived issues are read-only until restored. Priority defaults to none and status defaults to the team's first unstarted status by position and UUID. Team keys and issue teams remain immutable. UUIDs are canonical internal identities; workspace-scoped identifiers such as ENG-42 also resolve details. Browser links use `/<workspace-slug>/issue/<issue-key>/<issue-title-slug>`. The issue key identifies the issue; the title suffix is cosmetic and does not participate in lookup. Key-only quick links and stale-title links redirect to the current title using history replacement. Saving or adopting a changed title updates the URL without remounting the editor or clearing drafts. Slugs use Unicode NFKD normalization, remove combining marks, lowercase letters, and replace runs of non-letter/non-digit characters with hyphens; a symbol-only title uses `issue`.

Active workspace members can create and edit issues across the workspace. Archived issues remain readable through direct links and an explicit archived list; property edits require restoration. Archive and restore are exclusive patch operations. Physical deletion is unavailable. Membership removal clears assignments, including archived issues, increments their versions, and records activity against the acting administrator in the removal transaction.

`POST /api/v1/workspaces/{workspace_id}/issues` returns 201. `GET` on that collection returns active issues, or archived issues only with `archived=true`, ordered by creation time descending and UUID descending. Optional `team_id` restricts the collection to a team in that workspace; a missing or foreign team returns 404 and a malformed UUID returns 400. The browser sidebar has Home and Issues links per team, following the supplied UI references. The team root `/<workspace-slug>/team/<team-slug>` shows its home; `/all` lists active issues, `/archive` lists archived issues, and `/new` opens creation. Lists group the selected team’s issues by workflow status and preserve that scope through archive navigation and creation defaults. Pagination and additional filters remain CP05 work. API details resolve by UUID or exact human identifier. Browser details use issue keys and link back to their owning team's list.

`PATCH /api/v1/workspaces/{workspace_id}/issues/{issue_ref}` requires a positive `expected_version` and accepts only title, description, status_id, priority, assignee_membership_id, and archived. Omitted properties stay unchanged. Explicit null clears the assignee; null is rejected for other patch properties. Unknown properties are rejected. An unchanged patch returns the current issue without incrementing its version or recording activity. A stale version returns 409 `{code, message, current_issue}` without applying any changes. The editor sends only changed properties and requires review before retrying a stale patch, preserving its draft. Property edits and archive/restore increment the issue version once and append activity in the same transaction.

Issue transactions acquire a shared workspace lock before validating the session and membership, then lock the issue for an update or the team counter for number allocation. These operations avoid the installation-wide CP02 gate and account/session write locks. CP02 membership removal and account disablement hold exclusive workspace locks, preventing assignments to a membership that is being removed or disabled. Session validity is checked at the operation's authorization point; issue reads do not update the session's last-activity timestamp. Activity snapshots preserve actor, status, and assignee display names plus before/after property values for CP06.
