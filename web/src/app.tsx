import {
  createContext,
  type FormEvent,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useId,
  useState,
} from "react";
import {
  Link,
  Navigate,
  Route,
  Routes,
  useLocation,
  useNavigate,
  useParams,
} from "react-router";
import { api } from "./api/client";
import type { components } from "./api/schema";
import { SystemPage } from "./system-page";

type Schema = components["schemas"];
type Account = Schema["AccountResponse"];
type Bootstrap = Schema["BootstrapInfo"];

class RequestFailure extends Error {}

export function unwrap<T>(result: {
  data?: T;
  error?: Schema["ApiError"];
  response: Response;
}): T {
  if (!result.response.ok || result.data === undefined)
    throw new RequestFailure(
      result.error?.message ?? "The request failed. Try again.",
    );
  return result.data;
}
const Auth = createContext<{
  account: Account | null;
  bootstrap: Bootstrap;
  reload: () => Promise<Account | null>;
} | null>(null);
function useAuth() {
  const value = useContext(Auth);
  if (!value) throw new Error("Missing authentication context");
  return value;
}
function errorMessage(error: unknown) {
  return error instanceof RequestFailure
    ? error.message
    : "Could not reach the server. Check your connection and try again.";
}

export function App() {
  const [account, setAccount] = useState<Account | null>(null);
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [error, setError] = useState("");
  const reload = useCallback(async () => {
    const [setup, current] = await Promise.all([
      api.GET("/api/v1/auth/bootstrap"),
      api.GET("/api/v1/auth/me"),
    ]);
    setBootstrap(unwrap(setup));
    if (current.response.status !== 401) unwrap(current);
    const account = current.data ?? null;
    setAccount(account);
    return account;
  }, []);
  useEffect(() => {
    reload().catch((error) => setError(errorMessage(error)));
  }, [reload]);
  if (!bootstrap)
    return (
      <div className="shell">
        <header>
          <Link className="brand" to="/">
            Convex
            <span className="brand-dot" />
          </Link>
        </header>
        <main>
          {error ? (
            <>
              <p role="alert">{error}</p>
              <button
                type="button"
                onClick={() => {
                  setError("");
                  reload().catch((error) => setError(errorMessage(error)));
                }}
              >
                Try again
              </button>
            </>
          ) : (
            <p role="status">Connecting to Convex…</p>
          )}
        </main>
      </div>
    );
  return (
    <Auth.Provider value={{ account, bootstrap, reload }}>
      <div className="shell">
        <header>
          <Link className="brand" to="/">
            Convex
            <span className="brand-dot" />
          </Link>
          <nav aria-label="Main navigation">
            <Link to="/system">System status</Link>
            {account && (
              <>
                <Link to="/account">{account.user.display_name}</Link>
                <button
                  type="button"
                  onClick={async () => {
                    try {
                      unwrap(await api.POST("/api/v1/auth/logout"));
                      await reload();
                    } catch (error) {
                      setError(errorMessage(error));
                    }
                  }}
                >
                  Sign out
                </button>
              </>
            )}
          </nav>
        </header>
        <main>
          {error && (
            <p role="alert" className="notice error">
              {error}
            </p>
          )}
          <Routes>
            <Route
              path="/"
              element={
                bootstrap.setup_required ? (
                  <Navigate to="/setup" replace />
                ) : account ? (
                  <Home />
                ) : (
                  <Navigate to="/login" replace />
                )
              }
            />
            <Route path="/setup" element={<Setup />} />
            <Route path="/login" element={<Login />} />
            <Route path="/invite" element={<InvitationAcceptance />} />
            <Route path="/recovery" element={<Recovery />} />
            <Route path="/reset" element={<Reset />} />
            <Route
              path="/account"
              element={
                account ? <AccountSettings /> : <Navigate to="/login" replace />
              }
            />
            <Route
              path="/workspaces/:workspaceId"
              element={
                account ? (
                  <WorkspaceSettings />
                ) : (
                  <Navigate to="/login" replace />
                )
              }
            />
            <Route path="/system" element={<SystemPage />} />
            <Route
              path="*"
              element={
                <>
                  <h1>Page not found</h1>
                  <p>This page does not exist.</p>
                  <Link to="/">Return home</Link>
                </>
              }
            />
          </Routes>
        </main>
        <footer>Convex · Self-hosted issue tracking</footer>
      </div>
    </Auth.Provider>
  );
}

function Form({
  children,
  submit,
  action,
  onSuccess,
}: {
  children: ReactNode;
  submit: string;
  action: (form: FormData) => Promise<string | undefined>;
  onSuccess?: () => void;
}) {
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  async function handle(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setError("");
    setMessage("");
    const form = event.currentTarget;
    try {
      const message = await action(new FormData(form));
      setMessage(message ?? "Saved.");
      onSuccess?.();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
    }
  }
  return (
    <form onSubmit={handle}>
      <fieldset disabled={pending}>
        {children}
        <button className="primary" type="submit">
          {pending ? "Working…" : submit}
        </button>
      </fieldset>
      {error && (
        <p role="alert" className="notice error">
          {error}
        </p>
      )}
      {message && (
        <p role="status" className="notice">
          {message}
        </p>
      )}
    </form>
  );
}
function Field({
  name,
  label,
  type = "text",
  autoComplete,
  value,
  hint,
  required = true,
}: {
  name: string;
  label: string;
  type?: string;
  autoComplete?: string;
  value?: string;
  hint?: string;
  required?: boolean;
}) {
  const id = useId();
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <input
        id={id}
        name={name}
        type={type}
        autoComplete={autoComplete}
        defaultValue={value}
        required={required}
        minLength={type === "password" ? 12 : undefined}
        maxLength={type === "password" ? 128 : type === "email" ? 254 : 100}
        aria-describedby={hint ? `${id}-hint` : undefined}
      />
      {hint && <small id={`${id}-hint`}>{hint}</small>}
    </div>
  );
}
function text(form: FormData, key: string) {
  return String(form.get(key) ?? "");
}
function Credentials({ creating = false }: { creating?: boolean }) {
  return (
    <>
      {creating && (
        <Field name="display_name" label="Your name" autoComplete="name" />
      )}
      <Field name="email" label="Email" type="email" autoComplete="email" />
      <Field
        name="password"
        label="Password"
        type="password"
        autoComplete={creating ? "new-password" : "current-password"}
        hint={creating ? "Use 12–128 characters." : undefined}
      />
    </>
  );
}

function Setup() {
  const { bootstrap, reload } = useAuth();
  const navigate = useNavigate();
  if (!bootstrap.setup_required) return <Navigate to="/" replace />;
  return (
    <>
      <h1>Make room for your team.</h1>
      <p className="intro">
        Create your administrator account, workspace, and first team.
      </p>
      {bootstrap.setup_enabled ? (
        <Form
          submit="Create workspace"
          action={async (form) => {
            unwrap(
              await api.POST("/api/v1/auth/setup", {
                body: {
                  setup_token: text(form, "setup_token"),
                  email: text(form, "email"),
                  password: text(form, "password"),
                  display_name: text(form, "display_name"),
                  workspace_name: text(form, "workspace_name"),
                  team_name: text(form, "team_name"),
                  team_key: text(form, "team_key"),
                },
              }),
            );
            await reload();
            navigate("/");
            return undefined;
          }}
        >
          <Field
            name="setup_token"
            label="Installation setup credential"
            autoComplete="off"
            hint="Provided by your installation operator."
          />
          <Credentials creating />
          <div className="form-section">
            <Field name="workspace_name" label="Workspace name" />
            <div className="field-pair">
              <Field name="team_name" label="First team name" />
              <Field
                name="team_key"
                label="Team key"
                hint="2–10 letters or numbers, e.g. ENG."
              />
            </div>
          </div>
        </Form>
      ) : (
        <p className="notice">
          Setup is waiting for your installation operator to configure a setup
          credential and restart the server.
        </p>
      )}
    </>
  );
}
function Login() {
  const { account, bootstrap, reload } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const returnTo = location.search === "?return=invite" ? "/invite" : "/";
  if (account) return <Navigate to={returnTo} replace />;
  if (bootstrap.setup_required) return <Navigate to="/setup" replace />;
  return (
    <>
      <h1>Welcome back.</h1>
      <p className="intro">
        Sign in to your workspace. Joining for the first time? Open the
        invitation link from your admin.
      </p>
      <Form
        submit="Sign in"
        action={async (form) => {
          unwrap(
            await api.POST("/api/v1/auth/login", {
              body: {
                email: text(form, "email"),
                password: text(form, "password"),
              },
            }),
          );
          await reload();
          navigate(returnTo);
          return undefined;
        }}
      >
        <Credentials />
      </Form>
      <Link className="after-form" to="/recovery">
        Forgot your password?
      </Link>
    </>
  );
}
function Home() {
  const { account, reload } = useAuth();
  const navigate = useNavigate();
  if (!account) return null;
  return (
    <>
      <h1>Your workspaces</h1>
      <p className="intro">
        People, teams, and a shared place to organize work.
      </p>
      <ul className="workspace-list">
        {account.workspaces.map((space) => (
          <li key={space.id}>
            <Link to={`/workspaces/${space.id}`}>
              <strong>{space.name}</strong>
              <span>{space.role}</span>
            </Link>
          </li>
        ))}
      </ul>
      {account.workspaces.length === 0 && (
        <p className="notice">
          You have no active workspace memberships. Ask a workspace admin for an
          invitation.
        </p>
      )}
      {account.user.is_instance_admin && (
        <section>
          <h2>Create another workspace</h2>
          <Form
            submit="Create workspace"
            action={async (form) => {
              const space = unwrap(
                await api.POST("/api/v1/workspaces", {
                  body: {
                    name: text(form, "name"),
                    team_name: text(form, "team_name"),
                    team_key: text(form, "team_key"),
                  },
                }),
              );
              await reload();
              navigate(`/workspaces/${space.id}`);
              return undefined;
            }}
          >
            <Field name="name" label="Workspace name" />
            <div className="field-pair">
              <Field name="team_name" label="First team name" />
              <Field name="team_key" label="Team key" />
            </div>
          </Form>
        </section>
      )}
    </>
  );
}

function InvitationAcceptance() {
  const { account, reload } = useAuth();
  const navigate = useNavigate();
  const [token] = useState(() => {
    const token =
      window.location.hash.slice(1) ||
      sessionStorage.getItem("convex-invitation") ||
      "";
    if (token) sessionStorage.setItem("convex-invitation", token);
    window.history.replaceState(null, "", "/invite");
    return token;
  });
  const [preview, setPreview] = useState<Schema["InvitePreview"] | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!token) {
      setError(
        "Open the complete invitation link shared by your workspace admin.",
      );
      return;
    }
    let active = true;
    api
      .POST("/api/v1/invitations/preview", { body: { token } })
      .then(unwrap)
      .then((data) => {
        if (active) setPreview(data);
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      });
    return () => {
      active = false;
    };
  }, [token]);
  return (
    <>
      <h1>
        {preview ? `Join ${preview.workspace_name}` : "Workspace invitation"}
      </h1>
      {error ? (
        <p role="alert" className="notice error">
          {error}
        </p>
      ) : preview ? (
        <>
          <p className="intro">
            You’ve been invited as a workspace {preview.role}.{" "}
            {account
              ? `Continue as ${account.user.display_name}.`
              : "Create your account to join, or sign in with your existing account."}
          </p>
          <Form
            submit="Join workspace"
            action={async (form) => {
              const space = unwrap(
                await api.POST("/api/v1/invitations/accept", {
                  body: {
                    token,
                    ...(account
                      ? {}
                      : {
                          email: text(form, "email"),
                          password: text(form, "password"),
                          display_name: text(form, "display_name"),
                        }),
                  },
                }),
              );
              sessionStorage.removeItem("convex-invitation");
              await reload();
              navigate(`/workspaces/${space.id}`);
              return undefined;
            }}
          >
            {!account && <Credentials creating />}
          </Form>
          {!account && (
            <Link className="after-form" to="/login?return=invite">
              Already have an account? Sign in
            </Link>
          )}
        </>
      ) : (
        <p role="status">Checking your invitation…</p>
      )}
    </>
  );
}
function Recovery() {
  const { bootstrap } = useAuth();
  return (
    <>
      <h1>Recover your account</h1>
      <p className="intro">
        {bootstrap.mail_available
          ? "Request a password reset link for your local account."
          : "Email recovery is unavailable on this installation. Ask your installation operator for a password reset link."}
      </p>
      {bootstrap.mail_available && (
        <Form
          submit="Request reset link"
          action={async (form) =>
            unwrap(
              await api.POST("/api/v1/auth/recovery", {
                body: { email: text(form, "email") },
              }),
            ).message
          }
        >
          <Field name="email" label="Email" type="email" autoComplete="email" />
        </Form>
      )}
      <Link className="after-form" to="/login">
        Return to sign in
      </Link>
    </>
  );
}
function Reset() {
  const [token] = useState(() => {
    const value = window.location.hash.slice(1);
    return value;
  });
  useEffect(() => {
    window.history.replaceState(null, "", "/reset");
  }, []);
  const [done, setDone] = useState(false);
  const { reload } = useAuth();
  return (
    <>
      <h1>Choose a new password</h1>
      {done ? (
        <>
          <p role="status">
            Your password was reset and all sessions were signed out.
          </p>
          <Link to="/login">Sign in</Link>
        </>
      ) : token ? (
        <Form
          submit="Reset password"
          action={async (form) => {
            const result = unwrap(
              await api.POST("/api/v1/auth/reset", {
                body: { token, password: text(form, "password") },
              }),
            );
            await reload();
            setDone(true);
            return result.message;
          }}
        >
          <Field
            name="password"
            label="New password"
            type="password"
            autoComplete="new-password"
            hint="Use 12–128 characters."
          />
        </Form>
      ) : (
        <>
          <p className="notice">
            Open the complete reset link from your email or installation
            operator. Reset links expire after 30 minutes.
          </p>
          <Link to="/recovery">Get a new reset link</Link>
        </>
      )}
    </>
  );
}

function WorkspaceSettings() {
  const { workspaceId = "" } = useParams();
  const { account, reload, bootstrap } = useAuth();
  const [detail, setDetail] = useState<Schema["WorkspaceDetail"] | null>(null);
  const [invitations, setInvitations] = useState<Schema["Invitation"][]>([]);
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    const detail = unwrap(
      await api.GET("/api/v1/workspaces/{workspace_id}", {
        params: { path: { workspace_id: workspaceId } },
      }),
    );
    setDetail(detail);
    if (detail.workspace.role !== "member")
      setInvitations(
        unwrap(
          await api.GET("/api/v1/workspaces/{workspace_id}/invitations", {
            params: { path: { workspace_id: workspaceId } },
          }),
        ),
      );
  }, [workspaceId]);
  useEffect(() => {
    setDetail(null);
    setError("");
    load().catch((error) => setError(errorMessage(error)));
  }, [load]);
  const [link, setLink] = useState<Schema["InviteLink"] | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);
  const [copyMessage, setCopyMessage] = useState("");
  if (error)
    return (
      <>
        <h1>Workspace unavailable</h1>
        <p role="alert">{error}</p>
        <Link to="/">Return to workspaces</Link>
      </>
    );
  if (!detail || !account) return <p role="status">Loading workspace…</p>;
  const owner = detail.workspace.role === "owner";
  const manage = owner || detail.workspace.role === "admin";
  const params = { path: { workspace_id: workspaceId } };
  async function changeMember(member: Schema["Member"], role: string | null) {
    unwrap(
      await api.POST("/api/v1/workspaces/{workspace_id}/members", {
        params,
        body: { membership_id: member.id, role, remove: role === null },
      }),
    );
    setRemoving(null);
    if (member.user_id === account?.user.id) await reload();
    else await load();
  }
  return (
    <>
      <Link className="back" to="/">
        Workspaces
      </Link>
      <h1>{detail.workspace.name}</h1>
      <p className="intro">
        Your role: {detail.workspace.role}. Members can access every team in
        this workspace.
      </p>
      {manage && (
        <section>
          <h2>Workspace settings</h2>
          <Form
            key={detail.workspace.name}
            submit="Save workspace name"
            action={async (form) => {
              const result = unwrap(
                await api.POST("/api/v1/workspaces/{workspace_id}/rename", {
                  params,
                  body: { name: text(form, "name") },
                }),
              );
              await load();
              await reload();
              return result.message;
            }}
          >
            <Field
              name="name"
              label="Workspace name"
              value={detail.workspace.name}
            />
          </Form>
        </section>
      )}
      <section>
        <h2>
          People <span className="count">{detail.members.length}</span>
        </h2>
        <ul className="people-list">
          {detail.members.map((member) => (
            <li key={member.id}>
              <div>
                <strong>{member.display_name}</strong>
                <span>
                  {member.role}
                  {member.disabled ? " · Account disabled" : ""}
                </span>
              </div>
              {manage && (owner || member.role === "member") && (
                <div className="member-actions">
                  <Form
                    submit="Save role"
                    action={async (form) => {
                      await changeMember(member, text(form, "role"));
                      return "Role saved. Their sessions were revoked.";
                    }}
                  >
                    <label>
                      <span className="sr-only">
                        Role for {member.display_name}
                      </span>
                      <select name="role" defaultValue={member.role}>
                        <option value="member">Member</option>
                        {owner && (
                          <>
                            <option value="admin">Admin</option>
                            <option value="owner">Owner</option>
                          </>
                        )}
                      </select>
                    </label>
                  </Form>
                  {removing === member.id ? (
                    <Form
                      submit="Confirm removal"
                      action={async () => {
                        await changeMember(member, null);
                        return "Membership removed.";
                      }}
                    >
                      <p>Remove {member.display_name} from this workspace?</p>
                      <button type="button" onClick={() => setRemoving(null)}>
                        Cancel
                      </button>
                    </Form>
                  ) : (
                    <button
                      type="button"
                      onClick={() => setRemoving(member.id)}
                    >
                      Remove
                    </button>
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
      </section>
      {manage && (
        <section>
          <h2>Invite a teammate</h2>
          <p>
            Links work once and expire after seven days. Share only with the
            intended teammate.
          </p>
          <Form
            submit="Create invitation"
            action={async (form) => {
              const result = unwrap(
                await api.POST(
                  "/api/v1/workspaces/{workspace_id}/invitations",
                  {
                    params,
                    body: {
                      role: text(form, "role"),
                      deliver_to: text(form, "deliver_to") || null,
                    },
                  },
                ),
              );
              setLink(result);
              await load();
              return text(form, "deliver_to")
                ? result.email_delivered
                  ? "Invitation sent by email."
                  : "Email delivery failed. Share the link below directly."
                : "Invitation ready to share.";
            }}
          >
            <label className="field">
              <span>Invitation role</span>
              <select name="role">
                <option value="member">Member</option>
                {owner && <option value="admin">Admin</option>}
              </select>
            </label>
            {bootstrap.mail_available && (
              <Field
                name="deliver_to"
                label="Email delivery (optional)"
                type="email"
                required={false}
                hint="This sends a shared link; it does not restrict who can accept it."
              />
            )}
          </Form>
          {link && (
            <div className="invite-link">
              <label className="field">
                <span>Single-use invitation link</span>
                <input
                  readOnly
                  value={link.url}
                  onFocus={(event) => event.currentTarget.select()}
                />
              </label>
              <button
                type="button"
                onClick={async () => {
                  try {
                    await navigator.clipboard.writeText(link.url);
                    setCopyMessage("Copied");
                  } catch {
                    setCopyMessage(
                      "Select and copy the invitation link manually.",
                    );
                  }
                }}
              >
                Copy link
              </button>
              <p role="status">{copyMessage}</p>
            </div>
          )}
          {invitations.length > 0 && (
            <ul className="invitation-list">
              {invitations.map((invite) => (
                <li key={invite.id}>
                  <span>
                    {invite.role} ·{" "}
                    {invite.accepted_at
                      ? "Accepted"
                      : invite.revoked_at
                        ? "Revoked"
                        : new Date(invite.expires_at) < new Date()
                          ? "Expired"
                          : `Expires ${new Date(invite.expires_at).toLocaleDateString()}`}
                  </span>
                  {!invite.accepted_at && !invite.revoked_at && (
                    <Form
                      submit="Revoke"
                      action={async () => {
                        const result = unwrap(
                          await api.POST(
                            "/api/v1/workspaces/{workspace_id}/invitations/revoke",
                            { params, body: { invitation_id: invite.id } },
                          ),
                        );
                        await load();
                        return result.message;
                      }}
                    >
                      <span className="sr-only">Revoke invitation</span>
                    </Form>
                  )}
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
      <section>
        <h2>Teams</h2>
        {detail.teams.map((team) => (
          <div className="team" key={team.id}>
            <div className="section-heading">
              <h3>
                {team.name} <span className="team-key">{team.key}</span>
              </h3>
            </div>
            <ol className="workflow" aria-label={`${team.name} workflow`}>
              {team.statuses.map((status) => (
                <li key={status.id}>
                  {status.name}
                  <small>{status.category}</small>
                </li>
              ))}
            </ol>
            <p className="muted">
              New issues will start in{" "}
              {
                team.statuses.find((status) => status.category === "unstarted")
                  ?.name
              }
              . Team keys stay fixed to preserve issue identifiers.
            </p>
            {manage && (
              <Form
                key={team.name}
                submit="Save team name"
                action={async (form) => {
                  const result = unwrap(
                    await api.POST(
                      "/api/v1/workspaces/{workspace_id}/teams/{team_id}/rename",
                      {
                        params: {
                          path: { workspace_id: workspaceId, team_id: team.id },
                        },
                        body: { name: text(form, "name") },
                      },
                    ),
                  );
                  await load();
                  return result.message;
                }}
              >
                <Field
                  name="name"
                  label={`Name for ${team.key}`}
                  value={team.name}
                />
              </Form>
            )}
          </div>
        ))}
        {manage && (
          <>
            <h3>Create a team</h3>
            <Form
              submit="Create team"
              action={async (form) => {
                const result = unwrap(
                  await api.POST("/api/v1/workspaces/{workspace_id}/teams", {
                    params,
                    body: { name: text(form, "name"), key: text(form, "key") },
                  }),
                );
                await load();
                return result.message;
              }}
            >
              <div className="field-pair">
                <Field name="name" label="Team name" />
                <Field
                  name="key"
                  label="Team key"
                  hint="2–10 letters or numbers, e.g. ENG."
                />
              </div>
            </Form>
          </>
        )}
      </section>
    </>
  );
}

function AccountSettings() {
  const { account, reload } = useAuth();
  const [sessions, setSessions] = useState<Schema["SessionInfo"][]>([]);
  const [accounts, setAccounts] = useState<Schema["AdminAccount"][]>([]);
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    setSessions(unwrap(await api.GET("/api/v1/auth/sessions")));
    if (account?.user.is_instance_admin)
      setAccounts(unwrap(await api.GET("/api/v1/admin/accounts")));
  }, [account?.user.is_instance_admin]);
  useEffect(() => {
    load().catch((error) => setError(errorMessage(error)));
  }, [load]);
  return (
    <>
      <Link className="back" to="/">
        Workspaces
      </Link>
      <h1>Your account</h1>
      {error && <p role="alert">{error}</p>}
      <section>
        <h2>Change password</h2>
        <Form
          submit="Change password"
          action={async (form) => {
            const result = unwrap(
              await api.POST("/api/v1/auth/password", {
                body: {
                  current_password: text(form, "current_password"),
                  new_password: text(form, "new_password"),
                },
              }),
            );
            await reload();
            await load();
            return result.message;
          }}
        >
          <Field
            name="current_password"
            label="Current password"
            type="password"
            autoComplete="current-password"
          />
          <Field
            name="new_password"
            label="New password"
            type="password"
            autoComplete="new-password"
            hint="Use 12–128 characters. Changing your password signs out all other sessions."
          />
        </Form>
      </section>
      <section>
        <h2>Active sessions</h2>
        <ul className="invitation-list">
          {sessions.map((session) => (
            <li key={session.id}>
              <div>
                <strong>
                  {session.current ? "This browser" : "Another session"}
                </strong>
                <span>
                  Signed in {new Date(session.created_at).toLocaleString()}
                </span>
                <span>
                  Expires {new Date(session.expires_at).toLocaleString()}
                </span>
              </div>
              <Form
                submit="Sign out session"
                action={async () => {
                  const result = unwrap(
                    await api.POST("/api/v1/auth/sessions/revoke", {
                      body: { session_id: session.id },
                    }),
                  );
                  if (session.current) await reload();
                  else await load();
                  return result.message;
                }}
              >
                <span className="sr-only">Sign out this session</span>
              </Form>
            </li>
          ))}
        </ul>
      </section>
      {account?.user.is_instance_admin && (
        <section>
          <h2>Installation accounts</h2>
          <p>
            Disabling an account immediately revokes all its sessions. Workspace
            memberships remain in place for recovery.
          </p>
          <ul className="people-list">
            {accounts.map((user) => (
              <li key={user.id}>
                <div>
                  <strong>{user.display_name}</strong>
                  <span>
                    {user.email} · {user.disabled ? "Disabled" : "Active"}
                  </span>
                </div>
                {user.id !== account.user.id && (
                  <Form
                    submit={
                      user.disabled ? "Enable account" : "Disable account"
                    }
                    action={async () => {
                      const result = unwrap(
                        await api.POST("/api/v1/admin/accounts/disable", {
                          body: { user_id: user.id, disabled: !user.disabled },
                        }),
                      );
                      await load();
                      return result.message;
                    }}
                  >
                    <span className="sr-only">Manage this account</span>
                  </Form>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}
    </>
  );
}
