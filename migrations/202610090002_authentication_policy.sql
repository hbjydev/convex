CREATE TABLE convex.authentication_policy (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    fingerprint text,
    provider_revision text,
    exchange_revision text,
    oidc_issuer text,
    oidc_enabled boolean NOT NULL DEFAULT false,
    oidc_required boolean NOT NULL DEFAULT false,
    session_ttl_seconds bigint NOT NULL DEFAULT 604800
        CHECK (session_ttl_seconds BETWEEN 1 AND 2147483647),
    CHECK (NOT oidc_required OR oidc_enabled),
    CHECK (NOT oidc_enabled OR (oidc_issuer IS NOT NULL AND provider_revision IS NOT NULL))
);
INSERT INTO convex.authentication_policy DEFAULT VALUES;

CREATE TABLE convex.oidc_identities (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES convex.users(id),
    issuer text COLLATE "C" NOT NULL,
    subject text COLLATE "C" NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    unlinked_at timestamptz,
    UNIQUE (issuer, subject),
    UNIQUE (id, user_id)
);
CREATE UNIQUE INDEX oidc_identity_account_issuer ON convex.oidc_identities (user_id, issuer)
    WHERE unlinked_at IS NULL;

ALTER TABLE convex.sessions DROP CONSTRAINT sessions_login_method_check;
ALTER TABLE convex.sessions
    ALTER COLUMN expires_at DROP DEFAULT,
    ADD COLUMN oidc_identity_id uuid,
    ADD COLUMN provider_revision text,
    ADD CONSTRAINT sessions_identity_owner FOREIGN KEY (oidc_identity_id, user_id)
        REFERENCES convex.oidc_identities(id, user_id),
    ADD CONSTRAINT sessions_login_method_check CHECK (
        (login_method = 'local' AND oidc_identity_id IS NULL AND provider_revision IS NULL)
        OR (login_method = 'oidc' AND oidc_identity_id IS NOT NULL AND provider_revision IS NOT NULL)
    );

CREATE TABLE convex.oidc_flows (
    id uuid PRIMARY KEY,
    state_hash text NOT NULL UNIQUE,
    browser_hash text NOT NULL,
    intent text NOT NULL CHECK (intent IN ('login', 'invite', 'link', 'reauth')),
    exchange_revision text NOT NULL,
    nonce text,
    pkce_verifier text,
    user_id uuid REFERENCES convex.users(id),
    session_id uuid REFERENCES convex.sessions(id),
    invitation_id uuid REFERENCES convex.invitations(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL DEFAULT now() + interval '10 minutes',
    consumed_at timestamptz,
    CHECK (intent NOT IN ('link', 'reauth') OR (user_id IS NOT NULL AND session_id IS NOT NULL))
);
CREATE INDEX oidc_flows_expiry ON convex.oidc_flows(expires_at);
