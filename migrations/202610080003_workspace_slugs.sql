ALTER TABLE convex.workspaces ADD COLUMN slug text;

-- Existing workspaces get readable, deterministic URLs without name collisions.
-- Their UUID links remain supported by the browser router.
UPDATE convex.workspaces
SET slug = coalesce(
    nullif(trim(both '-' from left(regexp_replace(lower(name), '[^a-z0-9]+', '-', 'g'), 26)), ''),
    'workspace'
) || '-' || id::text;

ALTER TABLE convex.workspaces
    ALTER COLUMN slug SET NOT NULL,
    ADD CONSTRAINT workspaces_slug_key UNIQUE (slug),
    ADD CONSTRAINT workspaces_slug_format CHECK (
        length(slug) BETWEEN 3 AND 63 AND slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
        AND slug NOT IN ('account', 'api', 'assets', 'auth', 'invite', 'setup', 'system', 'workspaces')
    );
