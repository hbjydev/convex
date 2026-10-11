# Convex accounts and OIDC login

The first self-hosted release includes built-in accounts and configurable OpenID Connect login. Convex acts as an OIDC client to an existing identity provider. Both login methods resolve to the same internal user-account model, sessions, and workspace permission checks.

## First release direction

CP02 implements the local-account behavior recorded below. CP03 implements the provider behavior and account-method controls recorded below.

- Local login uses email and password. OIDC login appears when an operator configures a provider.
- Begin with one installation-level OIDC provider, configured through deployment settings: issuer URL, client ID, client secret, and display name. Keep secrets out of account and workspace responses.
- Bootstrap the first instance admin through a one-time installer-authorized setup flow. Public visitors cannot claim an uninitialized instance without the setup credential. Completing setup consumes that credential and creates the initial administrator account and session. The administrator then creates a workspace through the normal workspace creation page.
- Keep public signup disabled initially. Workspace owners/admins can create expiring, single-use invitation links that authorize joining their workspace through either supported login method.
- Admin-shared invitation links work without SMTP. Email delivery and email-based password recovery require configured mail. An email-bound invitation additionally requires proof of that email address; possession of a general invitation link is its explicit authorization mechanism.
- Provide an operator recovery command for existing local credentials when mail is unavailable. OIDC-only accounts recover through their provider; local password reset never silently adds a password to such an account.
- Keep a local instance-admin account available for operator recovery when the OIDC provider is unavailable. Instance-admin status manages installation settings; workspace membership separately governs issue access.

## Identity and account linking

An external login is identified by its exact issuer and subject (`iss`, `sub`), rather than by its email claim. These are the stable identity claims defined by [OIDC Core](https://openid.net/specs/openid-connect-core-1_0.html#ClaimStability).

Matching email addresses never automatically merge accounts. A signed-in user links another login method by freshly proving control of their existing method and the new method. A login identity already attached to another account cannot be claimed. Unlinking cannot remove the account's final usable login method.

An invited new user may create an account and accept the invitation in one flow. An existing user accepts it onto their current account after authenticating. Provider email domains and group claims grant no workspace roles in the first release. Removed memberships remain removed until explicitly restored by a workspace admin.

## Proposed persisted entities

| Entity | Main data and constraints |
| --- | --- |
| User account | Existing `users` record: immutable UUID, profile, instance-admin flag, and optional disabled timestamp. |
| Local credential | One row per user with a unique normalized login email, password hash, and change timestamp. Define email normalization explicitly before implementation. |
| OIDC identity | User reference, exact issuer and subject, and creation timestamp; unique `(issuer, subject)`. Profile/email claims are metadata. |
| Session | User reference, login method/identity reference, hashed opaque token, creation time, expiry, last activity, and optional revocation time. |
| Invitation | Workspace, inviter membership, proposed role, hashed token, expiry, optional email restriction, and acceptance/revocation data. Owner transfer remains an explicit separate action. |
| Recovery token | Existing local-credential reference, hashed token, expiry, and consumption timestamp. |
| Pending OIDC flow | Expiring, single-use browser-bound state, nonce, PKCE verifier, and intent: login or linking to a specific authenticated account. |

Invitation acceptance atomically consumes the invitation and creates or explicitly reactivates its membership. Validate the inviter's current authority and allowed role at acceptance. An invitation or recovery token can succeed only once, including under concurrent requests.

Password recovery atomically consumes its token, changes the existing local credential, and revokes the user's sessions. Password changes also revoke existing sessions; the newly authenticated user receives a fresh session.

## Login and session implementation

Use Argon2id for local password hashing, following [OWASP password-storage guidance](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html). Rate-limit authentication and recovery endpoints and use non-enumerating responses where account existence is private.

Perform OIDC authorization-code exchange in the Rust backend using PKCE with S256, state, and nonce. Validate token signature, issuer, audience, expiry, and nonce with a maintained OIDC library. PKCE guidance applies to web applications as well as public clients. [OAuth security best practice](https://www.rfc-editor.org/rfc/rfc9700.html#section-2.1.1)

Both flows issue Convex-owned, revocable server-side sessions through Secure, HttpOnly cookies, with an explicit SameSite policy and CSRF protection. Rotate sessions on authentication and privilege changes. Keep provider tokens on the backend and discard them once identity validation finishes, since the initial release makes no upstream API calls. [OWASP session guidance](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)

Convex logout revokes the local session. Provider-wide logout is separate behavior. Provider-side account disablement does not automatically invalidate existing Convex sessions; decide session lifetimes and reauthentication rules before release. Convex account disablement revokes all its sessions, and workspace membership is checked on every protected operation.

## Acceptance scenarios

1. A team can bootstrap, sign in locally, and invite another person without running an external identity provider or mail server.
2. An invited OIDC user joins the intended workspace and can use the same issue workflow as a local user.
3. Equal email claims across local and OIDC logins do not merge users or grant workspace access.
4. Linking after proving both methods preserves the user's memberships and existing issue authorship.
5. Expired, replayed, or browser-mismatched callbacks fail; login and linking flows cannot be interchanged.
6. A revoked session, disabled account, or removed membership cannot continue accessing protected workspace data.
7. Concurrent invitation acceptance or password recovery consumes its token once.

## Policies and implementation evidence

CP02 settles local recovery delivery and its original session lifetime in its implementation decisions below. CP03 investigation began on 2026-10-09. The [CP03 implementation plan](cp03-implementation.md) records the agreed choices below, code changes, and verification evidence; the [OIDC research](cp03-oidc-research.md) records primary-source protocol and dependency findings. CP03 is complete, including the real-provider outage and local operator recovery exercise. Multiple providers, group-based role mapping, automatic provisioning, and local MFA remain later capabilities.

## CP03 agreed decisions

Session lifetime, provider policy, OIDC login/callback, and account-method controls implement these decisions. The real-provider outage and local operator recovery checks also pass.

- **Provider scope and configuration:** one installation-level provider shared across workspaces, configured through environment variables for CP03. No provider administration UI in this checkpoint. Workspace membership remains independent of provider claims.
- **Enforcement:** operators can require OIDC for ordinary users. Enabling enforcement immediately revokes their existing local sessions; their next protected request requires OIDC authentication. Local instance administrators remain exempt for authentication and recovery.
- **Provider changes:** disabling/removing the provider or changing its issuer/client ID revokes its OIDC sessions and invalidates pending login flows. Rotating the client secret preserves existing sessions but invalidates pending flows, which must restart.
- **Provider outages and request checks:** existing eligible Convex sessions keep working until normal expiry during a temporary outage. Protected requests validate Convex session, account, membership, and policy state without contacting the provider. Provider calls are limited to login/reauthentication and controlled discovery/key refresh. New OIDC login/linking fails with a retryable error during an outage; required OIDC remains enforced and local administrator recovery remains available.
- **Session lifetime:** local and OIDC sessions use the same configurable absolute lifetime, defaulting to seven days, without sliding renewal. Use `CONVEX_SESSION_TTL_SECONDS` (default `604800`) and derive persisted session expiry and cookie Max-Age from the same value. Shortening the lifetime persistently caps existing sessions from their original creation time, so sessions already older than the new limit lose access on their next protected request. Increasing it applies to newly issued sessions only, without extending existing expiry or reviving expired sessions. This supersedes CP02's fixed-seven-day duration when implemented; method provenance remains available for enforcement/revocation, but does not select a different lifetime.
- **Adding local credentials:** when OIDC is optional, an OIDC-only account can add a local password after freshly authenticating with its attached provider identity. This preserves the same account and memberships. Ordinary users cannot add a password when OIDC is required; local instance administrators retain the option for recovery.
- **Logout:** Convex-only logout revokes the Convex session and pending actions bound to it, leaving the provider session active.

## CP02 implementation decisions

CP02 implements local accounts; CP03 adds OIDC configuration, linking, and provider enforcement.

- **Onboarding:** public signup is disabled. A random installer credential of at least 32 characters in `CONVEX_SETUP_TOKEN` authorizes first-admin setup. A locked installation row makes setup single-use, including concurrent requests. Setup creates one local instance admin and session in one transaction. After setup and on subsequent sign-ins, administrators without an owned workspace are directed to the workspace creation page. Creating a workspace gives them owner membership and creates its first team. A consumed installation never accepts setup again, even after a restart. Remove the environment credential after setup.
- **Accounts and email:** login identifiers are trimmed ASCII email addresses with the whole address lowercased; plus suffixes and dots are retained. Passwords contain 12–128 characters (at most 512 UTF-8 bytes) and use salted Argon2id PHC hashes with the [RustCrypto default parameters](https://docs.rs/argon2/0.5.3/argon2/struct.Params.html) (19 MiB, two iterations, one lane). Display names and workspace/team names contain 1–100 characters after trimming. An email is a login identifier, not a verified profile claim or workspace authorization mechanism.
- **Invitations:** every CP02 invitation is a general capability link, valid for seven days and one acceptance. Optional SMTP delivery sends that same link; the recipient address does not restrict acceptance. Email-bound invitations are not supported in CP02. Tokens stay in URL fragments and JSON bodies, never query strings. Owners may invite admins or members; admins may invite members. Acceptance rechecks the inviter's current active membership, account, and authority. Existing users sign in before acceptance; matching email never bypasses authentication. Removed memberships are reactivated in place by a new invitation. Existing active memberships cannot change roles through an invitation.
- **Roles:** members can read workspace members, teams, and workflow configuration. Owners and admins manage workspace names, teams, invitations, and ordinary members. Only owners grant/change owner or admin roles. An owner can grant ownership to another active member before giving up ownership; each operation preserves at least one active, enabled owner. Any signed-in user can create a workspace and becomes its owner. Only owners can delete workspaces, using an emailed, single-use code that expires after 10 minutes and an explicit acknowledgment. Email delivery and an email sign-in method are required. Instance administration alone grants no access to a workspace.
- **Teams:** workspace-wide visibility, immutable uppercase keys matching `[A-Z][A-Z0-9]{1,9}`, and five ordered initial statuses: Backlog, Todo, In Progress, Done, Canceled. The first unstarted status (Todo) is the default for issue creation. Team names can change; keys remain stable. Workflow editing and issue-dependent status replacement are deferred to the issue workflow, before configurable issue status changes are offered.
- **Sessions:** random opaque 64-character tokens (two UUIDv4 values), SHA-256 hashes in PostgreSQL, absolute configurable expiry (seven days by default), no sliding renewal. Cookies are host-only, HttpOnly, SameSite=Lax, path `/`, and Secure on HTTPS. `CONVEX_PUBLIC_URL` is an exact origin; non-loopback HTTP is rejected. Authentication rotates the browser session. Password changes revoke every old session and return a fresh one; recovery revokes sessions and requires login again. Logout and individual session controls revoke server-side sessions. Role changes, membership removal, invitation acceptance, disablement, and re-enablement revoke the affected account's sessions. Every protected request rechecks account state and active workspace membership.
- **CSRF:** every unsafe request, including setup, login, and recovery, must carry `Origin` equal to `CONVEX_PUBLIC_URL` and `X-Convex-CSRF: 1`. The frontend sends the custom header and browsers supply Origin. Cross-origin requests are not enabled through CORS. Scripted API clients must supply both headers. API responses use `Cache-Control: no-store`. Deployment must keep the web app and API on one origin.
- **Throttling:** PostgreSQL-backed fixed windows coordinate replicas: ten attempts per login email, recovery email, token, or setup bucket per 15 minutes, plus 120 authentication attempts across the installation per 15 minutes. Password hashing runs on blocking workers. Missing-account login verifies a dummy hash. Recovery responses are uniform; account lookup and SMTP delivery run outside the response path so mail latency does not disclose account existence. Expired rate buckets are pruned.
- **Transactions:** CP02 account and workspace control-plane transactions lock the singleton installation row first, then account/workspace/token rows. This deliberately serializes low-volume onboarding and administration across replicas and gives permission changes, session revocation, and token consumption a consistent lock order. CP04 issue operations must introduce scoped locks instead of using this installation gate. Account disablement and membership changes preserve durable records.
- **SMTP and recovery:** SMTP is optional. Without it, share invitation links directly and use the operator command below. With it, invitation delivery reports success/failure while still returning the link; a failed send can be recovered by sharing that link. Password recovery issues a single-use, 30-minute token for an existing enabled local credential only; no credential is created. Issuing another token or changing/resetting the password invalidates earlier tokens. Background recovery mail is best-effort and is not a durable queue: if a server stops before delivery, request another link. SMTP errors are logged without returning account-existence details. SMTP uses STARTTLS by default, with optional credentials; plaintext is restricted to loopback or the local Mailpit service.
- **Disablement:** instance admins can disable/enable other accounts. Self-disablement, removing the last active instance admin, and disabling a workspace's last active owner are rejected. Disabled users cannot log in or recover by email. An operator can explicitly enable an existing local account without restoring any removed memberships.

## First installation and operator recovery

1. Set `CONVEX_PUBLIC_URL` to the browser origin and `CONVEX_SETUP_TOKEN` to a securely generated secret of at least 32 characters, then start Convex.
2. Open the web app, enter that installation credential, and create the admin account, workspace, and first team. Store the local administrator password securely; it is independent of the later OIDC provider.
3. Remove `CONVEX_SETUP_TOKEN` from the environment and restart. The persisted installation marker prevents another setup with the old credential.
4. Open the workspace, create an invitation, and privately share its link with a teammate. They create a local account or sign in to an existing account before joining.

The operator command requires trusted database access. It never adds a local login method to an OIDC-only account, promotes an account, restores a removed membership, or resets first-run setup:

```sh
mise run account:recover person@example.com
# Explicitly re-enable a disabled local account as part of recovery:
mise run account:recover person@example.com --enable
```

It revokes all account sessions and prints a single-use recovery URL, valid for 30 minutes. Privately deliver it to the account holder. They choose a new password in the browser and sign in. Treat that printed URL as a temporary credential. Run the command with the same `DATABASE_URL` and `CONVEX_PUBLIC_URL` as the installation. Direct deployments can invoke the `recover-account` application binary with the same arguments and environment.

For optional local SMTP, run `mise run mail:up`, set the example SMTP variables in `.env`, and restart the API. Use `CONVEX_SMTP_HOST=127.0.0.1`, `CONVEX_SMTP_PORT=1025`, `CONVEX_SMTP_TLS=plain`, and a valid `CONVEX_SMTP_FROM`. Leave `CONVEX_SMTP_USERNAME` and `CONVEX_SMTP_PASSWORD` unset for Mailpit; blank usernames are treated as unauthenticated SMTP. Read delivered messages at <http://127.0.0.1:8025>. Production SMTP uses STARTTLS on port 587 by default; configure the host, from address, and any required username/password. Do not use the Mailpit inbox for production mail.

## Session configuration and CP03 foundation

`CONVEX_SESSION_TTL_SECONDS` controls the absolute lifetime of all newly issued Convex sessions, including session rotation. It defaults to `604800` (seven days), accepts integers from `1` through `2147483647`, and does not slide with activity. Invalid values prevent startup. Cookies and persisted expiry use the same setting. Shortening it caps existing sessions from their original creation time; increasing it never extends their stored expiry or restores expired sessions. A browser may retain an older cookie longer than a shortened server expiry, but that cookie cannot authorize access after expiry.

Before changing authentication settings, stop every application replica, update their environment variables consistently, then start them again. Replicas with identical authentication settings can run together. A replica with conflicting settings fails startup while an old replica holds its PostgreSQL policy lease. The active policy and all required expiry caps/revocations are committed before the new application begins serving. Each replica uses a dedicated PostgreSQL connection for that lease in addition to its request pool. If that connection fails, readiness and authentication return 503; restart the affected replica to restore its lease.

The CP03 foundation validates `CONVEX_OIDC_ISSUER_URL`, `CONVEX_OIDC_CLIENT_ID`, `CONVEX_OIDC_CLIENT_SECRET`, optional `CONVEX_OIDC_DISPLAY_NAME`, and boolean `CONVEX_OIDC_ENABLED`/`CONVEX_OIDC_REQUIRED`. A complete provider is enabled by default; enforcement defaults to false. The issuer must use HTTPS outside loopback development and have no credentials, query, or fragment. Required OIDC needs a configured, enabled provider. Configuration/discovery secrets are absent from account responses and no provider request is needed for policy activation, session validation, or administrator recovery.

OIDC login and invitation onboarding are implemented and tested against Keycloak 26.8.0. Register exactly `<CONVEX_PUBLIC_URL>/api/v1/auth/oidc/callback` as the provider's redirect URI, enable the authorization-code flow, and require S256 PKCE for the confidential client. Public bootstrap exposes only the enabled provider's display name and enforcement flag. Matching emails never link accounts. A new external identity needs an invitation; an attached identity resolves its existing account. Existing local users can link their provider from Account settings while OIDC is optional. Link their accounts before enforcing OIDC; ordinary local sessions are revoked when enforcement activates. See the [CP03 plan](cp03-implementation.md) for rollout and verification evidence.

`POST /api/v1/auth/oidc/start` requires the usual same-origin CSRF headers and accepts an optional invitation token in JSON. It returns an authorization URL and a host-only HttpOnly browser-binding cookie. Flow state expires after ten minutes, is bound to its exact public origin and login/invitation intent, and is consumed before code exchange. Starts from an authenticated session bind to that session; logout or revocation cancels them. The backend validates the signed ID token, issuer, single audience, authorized party if supplied, expiry, nonce, and issuance time. An upstream access-token hash is checked when supplied. Provider access/refresh tokens are discarded. Signature keys and discovery are cached for five minutes; an unknown signing key permits one refresh per 30 seconds, without repeating code exchange. Failed refreshes also respect that cooldown.

Discovery, keys, and token exchange reject redirects, limit each response to 1 MiB, and use a ten-second request timeout with a three-second connection timeout. Endpoints require HTTPS except loopback development. Operator-configured private-network HTTPS providers are permitted. Provider availability does not affect startup, offline contract export, local administrator access, or existing eligible Convex sessions.

The GET callback redirects login outcomes to `/auth/oidc?outcome=success|failed`, successful linking to `/account?method=linked`, and fresh method proofs to `/account?method=proof`; upstream error details, code, and state are absent from the destination. API responses use `no-store` and `Referrer-Policy: no-referrer`. Application request tracing records the path without its query. Configure reverse-proxy/access logs the same way: omit the callback query and avoid request/response body logging for authentication endpoints. For Nginx, use `$request_method` and `$uri` rather than `$request` or `$request_uri` in the access-log format. Also redact query strings in infrastructure tracing and error reporting; application redaction cannot control proxy logs.

### Development provider

Run `mise run oidc:up` for the pinned Keycloak image and checked-in realm. The canonical host/browser issuer is `http://127.0.0.1:8083/realms/convex`; use client ID `convex`, client secret `convex-development-client-secret`, and display name `Development SSO`. Add those settings to your development environment, then restart all API replicas. The realm allows exact callbacks on browser ports 5173 (normal development) and 5274 (OIDC browser tests). Use `invited-member` / `convex provider test password` only for local testing; its email intentionally matches the browser fixture's local administrator to verify account separation. It must receive a Convex invitation before its first login can create an account.

`mise run test:e2e:oidc` starts the provider and test database, uses API port 3103 and browser port 5274, and runs desktop/mobile onboarding, repeat login, administrator fallback, and sanitized callback failure. The API uses a fresh disposable database and required OIDC. `mise run oidc:reset` recreates the disposable provider after realm edits; import on startup otherwise leaves an existing realm unchanged. To stop just the provider, run `docker compose --profile oidc stop oidc-cp03`. The public fixture credentials and signing key under `crates/convex/tests/fixtures` are exclusively test data. Do not deploy the development provider or reuse its secrets in production.


### Changing login methods

Account settings lists attached methods and their installation-policy eligibility. Linking single sign-on requires the current password and fresh provider authentication; adding a password requires fresh authentication as the exact attached provider identity. Ordinary users cannot add a password when OIDC is required. Removing single sign-on requires the remaining local password; removing a password requires fresh proof of the remaining enabled provider. The last enabled instance administrator with local credentials must retain that password for operator recovery.

Sensitive provider flows request `prompt=login` and `max_age=0`. The callback requires `auth_time` within five minutes and no earlier than the flow's creation minus a 30-second clock tolerance. A local linking proof expires after five minutes too. Provider proof is held in a hashed, single-use database record bound to the account, source session, browser, operation, identity, and provider configuration; its HttpOnly cookie grants no session access. Proof expires no later than five minutes after provider authentication. A failed mutation such as an email conflict leaves proof available to retry within that deadline. Logout, session revocation, and incompatible policy changes invalidate pending proof.

A successful method change revokes all old account sessions and issues a fresh session through a proven eligible method, preserving the account and memberships. Removing a password deletes its recovery-token records, so old reset links cannot recreate it. Unlinked provider identities stay reserved to their original account: that account may reattach them, but another account cannot claim them.

`mise run test:e2e:methods` exercises linking, password removal and reattachment, and provider removal on desktop and mobile against the pinned provider with optional OIDC. Its second fixture user is `invited-member-mobile` with the same development password; each browser scenario uses a separate exact provider subject.


### Provider outage and operator recovery

Run `mise run test:e2e:outage` exclusively: it stops and restarts the disposable `oidc-cp03` service, so do not run other provider suites alongside it. Each desktop/mobile scenario uses the same isolated API/browser ports and a fresh suite database. It holds real provider-issued authorization codes before the callback, stops Keycloak, and checks sanitized login/linking failures, unchanged accounts and session expiry, required-OIDC enforcement, new local administrator sign-in, administration, the real `recover-account` binary, browser password reset, old administrator-session revocation, and recovery after provider restart. The provider is restarted in cleanup even when an assertion fails.

A temporary outage does not change installation policy. Existing eligible Convex sessions retain their original expiry; new provider authentication needs the provider to return. Use a local instance administrator for installation access. If its password is lost, run the documented [operator recovery command](#first-installation-and-operator-recovery) with trusted database access and reset the existing local account; this does not add local credentials to OIDC-only users or bypass enforcement for ordinary users. Stop all replicas and wait for their database connections to close before changing provider settings, then restart every replica with matching environment values.

Verified compatibility covers the pinned Keycloak 26.8.0 realm and Chromium on desktop and an iPhone 13 viewport. Other providers and browser engines are not covered by this checkpoint's interoperability evidence.
