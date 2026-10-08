# Convex accounts and OIDC login

The first self-hosted release includes built-in accounts and configurable OpenID Connect login. Convex acts as an OIDC client to an existing identity provider. Both login methods resolve to the same internal user-account model, sessions, and workspace permission checks.

## Proposed first release behavior

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

Confirm installation-level provider configuration versus per-workspace providers, recovery delivery, session lifetimes, and whether operators may enforce OIDC for ordinary users while retaining local operator recovery. Multiple providers, group-based role mapping, automatic provisioning, and local MFA are later capabilities unless brought into scope explicitly.
