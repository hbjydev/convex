ALTER TABLE convex.oidc_flows
    ADD COLUMN method_action text CHECK (method_action IN ('add_local', 'remove_local')),
    ADD COLUMN local_verified_at timestamptz;

ALTER TABLE convex.sessions ADD CONSTRAINT sessions_identity_user UNIQUE (id, user_id);
CREATE TABLE convex.method_proofs (
    id uuid PRIMARY KEY,
    token_hash text NOT NULL UNIQUE,
    user_id uuid NOT NULL REFERENCES convex.users(id),
    session_id uuid NOT NULL,
    oidc_identity_id uuid NOT NULL,
    action text NOT NULL CHECK (action IN ('add_local', 'remove_local')),
    browser_hash text NOT NULL,
    exchange_revision text NOT NULL,
    verified_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL DEFAULT now() + interval '5 minutes',
    FOREIGN KEY (session_id, user_id) REFERENCES convex.sessions(id, user_id),
    FOREIGN KEY (oidc_identity_id, user_id) REFERENCES convex.oidc_identities(id, user_id)
);
CREATE INDEX method_proofs_expiry ON convex.method_proofs(expires_at);
