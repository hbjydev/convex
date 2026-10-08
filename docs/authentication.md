# Convex accounts and OIDC login

The first self-hosted release includes built-in accounts and configurable OpenID Connect login. Convex acts as an OIDC client to an existing identity provider. Both login methods resolve to the same internal user-account model, sessions, and workspace permission checks.

## First release direction

CP02 implements the local-account behavior recorded below. Provider behavior remains a CP03 proposal.

- Local login uses email and password. OIDC login appears when an operator configures a provider.
- Begin with one installation-level OIDC provider, configured through deployment settings: issuer URL, client ID, client secret, and display name. Keep secrets out of account and workspace responses.
- Bootstrap the first instance admin through a one-time installer-authorized setup flow. Public visitors cannot claim an uninitialized instance without the setup credential. Completing setup consumes that credential and creates the initial workspace and owner membership.
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

## Policies still open

CP02 settles local recovery delivery and session lifetimes in its implementation decisions below. CP03 must settle installation-level versus per-workspace provider configuration and whether operators may enforce OIDC for ordinary users while retaining local operator recovery. Multiple providers, group-based role mapping, automatic provisioning, and local MFA remain later capabilities.

## CP02 implementation decisions

CP02 implements local accounts. OIDC configuration, linking, and provider enforcement remain CP03 work.

- **Onboarding:** public signup is disabled. A random installer credential of at least 32 characters in `CONVEX_SETUP_TOKEN` authorizes first-admin setup. A locked installation row makes setup single-use, including concurrent requests. Setup creates one local instance admin, a workspace owner membership, and the first team in one transaction. A consumed installation never accepts setup again, even after a restart. Remove the environment credential after setup.
- **Accounts and email:** login identifiers are trimmed ASCII email addresses with the whole address lowercased; plus suffixes and dots are retained. Passwords contain 12–128 characters (at most 512 UTF-8 bytes) and use salted Argon2id PHC hashes with the [RustCrypto default parameters](https://docs.rs/argon2/0.5.3/argon2/struct.Params.html) (19 MiB, two iterations, one lane). Display names and workspace/team names contain 1–100 characters after trimming. An email is a login identifier, not a verified profile claim or workspace authorization mechanism.
- **Invitations:** every CP02 invitation is a general capability link, valid for seven days and one acceptance. Optional SMTP delivery sends that same link; the recipient address does not restrict acceptance. Email-bound invitations are not supported in CP02. Tokens stay in URL fragments and JSON bodies, never query strings. Owners may invite admins or members; admins may invite members. Acceptance rechecks the inviter's current active membership, account, and authority. Existing users sign in before acceptance; matching email never bypasses authentication. Removed memberships are reactivated in place by a new invitation. Existing active memberships cannot change roles through an invitation.
- **Roles:** members can read workspace members, teams, and workflow configuration. Owners and admins manage workspace names, teams, invitations, and ordinary members. Only owners grant/change owner or admin roles. An owner can grant ownership to another active member before giving up ownership; each operation preserves at least one active, enabled owner. Workspace creation is restricted to instance admins. Instance administration alone grants no access to a workspace.
- **Teams:** workspace-wide visibility, immutable uppercase keys matching `[A-Z][A-Z0-9]{1,9}`, and five ordered initial statuses: Backlog, Todo, In Progress, Done, Canceled. The first unstarted status (Todo) is the default for issue creation. Team names can change; keys remain stable. Workflow editing and issue-dependent status replacement are deferred to the issue workflow, before configurable issue status changes are offered.
- **Sessions:** random opaque 64-character tokens (two UUIDv4 values), SHA-256 hashes in PostgreSQL, absolute seven-day expiry, no sliding renewal. Cookies are host-only, HttpOnly, SameSite=Lax, path `/`, and Secure on HTTPS. `CONVEX_PUBLIC_URL` is an exact origin; non-loopback HTTP is rejected. Authentication rotates the browser session. Password changes revoke every old session and return a fresh one; recovery revokes sessions and requires login again. Logout and individual session controls revoke server-side sessions. Role changes, membership removal, invitation acceptance, disablement, and re-enablement revoke the affected account's sessions. Every protected request rechecks account state and active workspace membership.
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

For optional local SMTP, run `mise run mail:up`, set the example SMTP variables in `.env`, and restart the API. Use `CONVEX_SMTP_HOST=127.0.0.1`, `CONVEX_SMTP_PORT=1025`, `CONVEX_SMTP_TLS=plain`, and a valid `CONVEX_SMTP_FROM`. Read delivered messages at <http://127.0.0.1:8025>. Production SMTP uses STARTTLS on port 587 by default; configure the host, from address, and any required username/password. Do not use the Mailpit inbox for production mail.
