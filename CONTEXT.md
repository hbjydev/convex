# Convex issue tracking

Convex is an open-source issue tracker for organizing work. These definitions establish the proposed language for its first issue-management slice.

## Language

**User account**:
A person's durable identity in Convex, shared across their login methods and workspace memberships.
_Avoid_: Workspace membership

**Workspace**:
A shared space containing teams and their work. A person's membership in a workspace determines their access to that workspace.
_Avoid_: Organization, tenant

**Team**:
A group within a workspace that owns issues and defines a workflow for them.
_Avoid_: Project, board

**Workspace membership**:
A person's participation in a workspace, including their access role and whether they are active. A departed membership remains part of the historical record.
_Avoid_: Account, team member

**Issue**:
A discrete item of work owned by one team, with a title, description, status, priority, and optional assignee.
_Avoid_: Ticket, task

**Issue identifier**:
A human-readable reference consisting of a team key and a sequence number, such as ENG-42. An issue retains its identifier throughout its life.
_Avoid_: Issue ID, ticket number

**Status**:
A named step in a team's workflow, such as In Progress. Each status belongs to a category: backlog, unstarted, started, completed, or canceled.
_Avoid_: State, stage

**Priority**:
An issue's relative urgency: none, urgent, high, medium, or low.
_Avoid_: Severity

**Assignee**:
The workspace member currently responsible for an issue. An issue may be unassigned.
_Avoid_: Owner

**Label**:
A reusable tag for classifying issues within a workspace. An issue can have several labels.
_Avoid_: Category

**Comment**:
A discussion contribution attached to an issue and attributed to its author.
_Avoid_: Note

**Activity entry**:
A record of a meaningful change to an issue, including who made it and when.
_Avoid_: Comment, notification
