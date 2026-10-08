CREATE TABLE convex.installation (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    initialized_at timestamptz
);
INSERT INTO convex.installation DEFAULT VALUES;
CREATE TABLE convex.users (
    id uuid PRIMARY KEY,
    display_name text NOT NULL CHECK (length(display_name) BETWEEN 1 AND 100),
    is_instance_admin boolean NOT NULL DEFAULT false,
    disabled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE convex.local_credentials (
    user_id uuid PRIMARY KEY REFERENCES convex.users(id),
    email text NOT NULL UNIQUE CHECK (length(email) BETWEEN 3 AND 254 AND email = lower(email)),
    password_hash text NOT NULL,
    changed_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE convex.workspaces (
    id uuid PRIMARY KEY,
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE convex.workspace_memberships (
    id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES convex.workspaces(id),
    user_id uuid NOT NULL REFERENCES convex.users(id),
    role text NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
    joined_at timestamptz NOT NULL DEFAULT now(),
    removed_at timestamptz,
    UNIQUE (workspace_id, user_id),
    UNIQUE (workspace_id, id)
);
CREATE TABLE convex.teams (
    id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES convex.workspaces(id),
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    key text NOT NULL CHECK (key ~ '^[A-Z][A-Z0-9]{1,9}$'),
    next_issue_number bigint NOT NULL DEFAULT 1 CHECK (next_issue_number > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, key),
    UNIQUE (workspace_id, id)
);
CREATE TABLE convex.workflow_statuses (
    id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL,
    team_id uuid NOT NULL,
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    category text NOT NULL CHECK (category IN ('backlog', 'unstarted', 'started', 'completed', 'canceled')),
    position integer NOT NULL CHECK (position >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (workspace_id, team_id) REFERENCES convex.teams(workspace_id, id),
    UNIQUE (team_id, name),
    UNIQUE (team_id, position),
    UNIQUE (workspace_id, team_id, id)
);
CREATE TABLE convex.sessions (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES convex.users(id),
    token_hash text NOT NULL UNIQUE,
    login_method text NOT NULL CHECK (login_method = 'local'),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL DEFAULT now() + interval '7 days',
    last_activity_at timestamptz NOT NULL DEFAULT now(),
    revoked_at timestamptz
);
CREATE INDEX sessions_user ON convex.sessions(user_id);
CREATE TABLE convex.invitations (
    id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES convex.workspaces(id),
    inviter_membership_id uuid NOT NULL,
    role text NOT NULL CHECK (role IN ('admin', 'member')),
    token_hash text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL DEFAULT now() + interval '7 days',
    accepted_at timestamptz,
    accepted_by uuid REFERENCES convex.users(id),
    revoked_at timestamptz,
    FOREIGN KEY (workspace_id, inviter_membership_id) REFERENCES convex.workspace_memberships(workspace_id, id)
);
CREATE TABLE convex.recovery_tokens (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES convex.local_credentials(user_id),
    token_hash text NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL DEFAULT now() + interval '30 minutes',
    consumed_at timestamptz
);
-- Shared across application replicas; no attacker-controlled raw emails are retained.
CREATE TABLE convex.auth_rate_limits (
    bucket text PRIMARY KEY,
    started_at timestamptz NOT NULL DEFAULT now(),
    attempts integer NOT NULL DEFAULT 1
);
