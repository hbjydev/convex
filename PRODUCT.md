# Convex product context

## Purpose

Convex is an open-source issue tracker for self-hosting teams. This description comes from README.md. Domain terms and delivery scope remain authoritative in [CONTEXT.md](CONTEXT.md) and [docs/checkpoints.md](docs/checkpoints.md).

## Users and tasks

Repository-derived context: installation administrators set up the instance and manage accounts; workspace owners and administrators manage teams, people, and invitations; members join workspaces. The current UI implements these account and workspace flows. Issue features are planned separately.

## Platform and stack

Web application with desktop and mobile layouts. React, TanStack Router, TypeScript, Vite, Tailwind CSS v4, shadcn/ui Nova with Radix, Lucide icons, and locally served Geist. Rust/Axum and PostgreSQL provide the existing API.

## Confirmed design preferences

The user requested a Linear-inspired redesign and use of shadcn's Sidebar. The current workspace leads the sidebar header, with settings and a nested workspace-switching menu, matching the user-provided Linear screenshots. Both light and dark themes must follow system preference. Workspace creation includes an editable, unique slug, and workspace browser URLs start at `<domain>/<slug>`. Initial setup opens the newly created workspace.

## Constraints

Preserve working account, invitation, workspace, and team flows and role-based permissions. Derive screens from implemented functionality and real API data. Preserve native form validation, pending states, errors, keyboard access, and mobile usability.
