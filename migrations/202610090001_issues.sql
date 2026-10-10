CREATE TABLE convex.issues (
    id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL,
    team_id uuid NOT NULL,
    number bigint NOT NULL CHECK (number > 0),
    title text NOT NULL CHECK (length(title) BETWEEN 1 AND 240 AND title = btrim(title)),
    description text NOT NULL DEFAULT '' CHECK (length(description) <= 50000),
    status_id uuid NOT NULL,
    priority text NOT NULL DEFAULT 'none' CHECK (priority IN ('none','urgent','high','medium','low')),
    assignee_membership_id uuid,
    creator_membership_id uuid NOT NULL,
    version bigint NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    archived_at timestamptz,
    archived_by_membership_id uuid,
    CHECK ((archived_at IS NULL) = (archived_by_membership_id IS NULL)),
    FOREIGN KEY (workspace_id, team_id) REFERENCES convex.teams(workspace_id,id),
    FOREIGN KEY (workspace_id, team_id, status_id) REFERENCES convex.workflow_statuses(workspace_id,team_id,id),
    FOREIGN KEY (workspace_id, assignee_membership_id) REFERENCES convex.workspace_memberships(workspace_id,id),
    FOREIGN KEY (workspace_id, creator_membership_id) REFERENCES convex.workspace_memberships(workspace_id,id),
    FOREIGN KEY (workspace_id, archived_by_membership_id) REFERENCES convex.workspace_memberships(workspace_id,id),
    UNIQUE (team_id,number),
    UNIQUE (workspace_id,id)
);
CREATE INDEX issues_workspace_list ON convex.issues(workspace_id,archived_at,created_at DESC,id DESC);
CREATE INDEX issues_assignee ON convex.issues(workspace_id,assignee_membership_id) WHERE assignee_membership_id IS NOT NULL;
CREATE TABLE convex.issue_activity (
    id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL,
    issue_id uuid NOT NULL,
    actor_membership_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('created','updated','archived','restored')),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY (workspace_id,issue_id) REFERENCES convex.issues(workspace_id,id),
    FOREIGN KEY (workspace_id,actor_membership_id) REFERENCES convex.workspace_memberships(workspace_id,id)
);
CREATE INDEX issue_activity_timeline ON convex.issue_activity(issue_id,created_at,id);
