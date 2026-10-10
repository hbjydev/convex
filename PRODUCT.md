# Convex product context

## Purpose

Convex is an open-source issue tracker for self-hosting teams. This description comes from README.md. Domain terms and delivery scope remain authoritative in [CONTEXT.md](CONTEXT.md) and [docs/checkpoints.md](docs/checkpoints.md).

## Users and tasks

Repository-derived context: installation administrators set up the instance and manage accounts; workspace owners and administrators manage teams, people, and invitations; members join workspaces. The current UI implements these account and workspace flows. CP04 adds issue creation, status-grouped lists, property editing, archival, and restoration for authenticated members.

## Platform and stack

Web application with desktop and mobile layouts. React, TanStack Router, TypeScript, Vite, Tailwind CSS v4, shadcn/ui Nova with Radix, Lucide icons, and locally served Geist. Rust/Axum and PostgreSQL provide the existing API.

## Confirmed design preferences

The user requested a Linear-inspired redesign and use of shadcn's Sidebar. The current workspace leads the sidebar header, with settings and a nested workspace-switching menu, matching the user-provided Linear screenshots. Both light and dark themes must follow system preference. Workspace creation includes an editable, unique slug, and workspace browser URLs start at `<domain>/<slug>`. Initial setup opens the newly created workspace.

The sidebar groups navigation by team, following the user's second reference. Each team has Home and Issues links; issue lists and creation defaults follow the selected team.

Workspace switching, All workspaces, and workspace settings are reached through the top dropdown; these links are not repeated in the main sidebar.

Issue details follow the user's Linear reference, with a borderless ProseMirror description editor styled by shadcn typeset and a separate column of editable property menus. Keep Markdown as the storage format. Save title and description changes automatically after a pause and on blur, and properties when selected; preserve drafts through failed requests and conflict review.

## Constraints

Preserve working account, invitation, workspace, and team flows and role-based permissions. Derive screens from implemented functionality and real API data. Preserve native form validation, pending states, errors, keyboard access, and mobile usability.
