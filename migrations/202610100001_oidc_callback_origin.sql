ALTER TABLE convex.oidc_flows ADD COLUMN public_origin text;
-- Old pending flows have no origin binding and cannot authorize callbacks.
DELETE FROM convex.oidc_flows;
