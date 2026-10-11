# Convex product context

## Purpose

Convex is an open-source issue tracker for self-hosting teams. This description comes from README.md. Domain terms and delivery scope remain authoritative in [CONTEXT.md](CONTEXT.md) and [docs/checkpoints.md](docs/checkpoints.md).

## Users and tasks

Repository-derived context: installation administrators set up the instance and manage accounts; workspace owners and administrators manage teams, people, and invitations; members join workspaces. The current UI implements these account and workspace flows. CP04 adds issue creation, status-grouped lists, property editing, archival, and restoration for authenticated members.

## Platform and stack

Web application with desktop and mobile layouts. React, TanStack Router, TypeScript, Vite, Tailwind CSS v4, shadcn/ui Nova with Radix, Lucide icons, and locally served Geist. Rust/Axum and PostgreSQL provide the existing API.

## Confirmed design preferences

The user requested a Linear-inspired redesign and use of shadcn's Sidebar. The current workspace leads the sidebar header, with settings and a nested workspace-switching menu, matching the user-provided Linear screenshots. Both light and dark themes must follow system preference. Workspace creation includes an editable, unique slug, and workspace browser URLs start at `<domain>/<slug>`. Initial setup creates the administrator account and signs them in. Administrators without an owned workspace are directed to the standalone workspace creation page after setup and login.

The sidebar groups navigation by team, following the user's second reference. Each team has Home and Issues links; issue lists and creation defaults follow the selected team.

Workspace switching, Create a workspace, and workspace settings are reached through the top dropdown; these links are not repeated in the main sidebar.

Issue details follow the user's Linear reference, with a borderless ProseMirror description editor and a separate column of editable property menus. Keep Markdown as the storage format. Save title and description changes automatically after a pause and on blur, and properties when selected; preserve drafts through failed requests and conflict review.

Use Tailwind v4 utilities at the React component that owns the markup. Share reusable styles through React components and component variants. Global CSS contains theme tokens and browser-wide defaults.

The root URL opens the last-used workspace’s first team, falling back to the first available workspace. There is no workspace directory. Workspace creation is a standalone, centered page available to every signed-in user; its first team uses the workspace name and a key derived from its first three Latin letters (Hobbit Hole → HOB, Convex → CON). Workspace settings expose owner-only permanent deletion with an emailed code and acknowledgment. Codes expire after 10 minutes; SMTP and an email sign-in method are required.

## Constraints

Preserve working account, invitation, workspace, and team flows and role-based permissions. Derive screens from implemented functionality and real API data. Preserve native form validation, pending states, errors, keyboard access, and mobile usability.
