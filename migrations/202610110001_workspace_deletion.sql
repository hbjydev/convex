CREATE TABLE convex.workspace_deletion_codes (
    workspace_id uuid NOT NULL REFERENCES convex.workspaces(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES convex.users(id),
    code_hash text NOT NULL,
    expires_at timestamptz NOT NULL DEFAULT now() + interval '10 minutes',
    PRIMARY KEY (workspace_id, user_id)
);
