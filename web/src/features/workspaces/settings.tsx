import { Link, useLocation, useRouteContext } from "@tanstack/react-router";
import { ArrowLeftIcon } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { useAuth } from "@/auth";
import { Field, Form, RoleField, text } from "@/components/app-form";
import { LoadingState, Notice } from "@/components/feedback";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  FieldGroup,
  FieldLabel,
  Field as UIField,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { errorMessage, unwrap } from "@/lib/requests";

type Schema = components["schemas"];
export function WorkspaceSettings() {
  const { hash } = useLocation();
  const { workspaceId } = useRouteContext({
    from: "/_app/$workspaceSlug",
  });
  const { account, reload, bootstrap } = useAuth();
  const [detail, setDetail] = useState<Schema["WorkspaceDetail"] | null>(null);
  const [invitations, setInvitations] = useState<Schema["Invitation"][]>([]);
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    if (!workspaceId)
      throw new Error(
        "This workspace is unavailable or you no longer have access.",
      );
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
  useEffect(() => {
    if (!detail || (hash !== "people" && hash !== "teams")) return;
    const frame = requestAnimationFrame(() => {
      document.getElementById(hash)?.scrollIntoView({ block: "start" });
    });
    return () => cancelAnimationFrame(frame);
  }, [detail, hash]);
  const [link, setLink] = useState<Schema["InviteLink"] | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);
  const [copyMessage, setCopyMessage] = useState("");
  if (error)
    return (
      <>
        <h1>Workspace unavailable</h1>
        <Notice error>{error}</Notice>
        <Link to="/">Return to workspaces</Link>
      </>
    );
  if (!detail || !account || !workspaceId)
    return <LoadingState>Loading workspace…</LoadingState>;
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
      <Button asChild variant="ghost" className="mb-6 w-fit">
        <Link to="/">
          <ArrowLeftIcon data-icon="inline-start" />
          Workspaces
        </Link>
      </Button>
      <h1>{detail.workspace.name}</h1>
      <p className="intro">
        Your role: {detail.workspace.role}. Members can access every team in
        this workspace.
      </p>
      <p className="break-all text-sm text-muted-foreground">
        Workspace URL: {window.location.host}/{detail.workspace.slug}
      </p>
      {manage && (
        <section className="settings-section">
          <Separator />
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
      <section className="settings-section" id="people">
        <Separator />
        <h2>
          People <Badge variant="secondary">{detail.members.length}</Badge>
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
                    inline
                    variant="outline"
                    submit="Save role"
                    action={async (form) => {
                      await changeMember(member, text(form, "role"));
                      return "Role saved. Their sessions were revoked.";
                    }}
                  >
                    <RoleField
                      label={`Role for ${member.display_name}`}
                      value={member.role}
                      owner={owner}
                      includeOwner
                      hiddenLabel
                    />
                  </Form>
                  {removing === member.id ? (
                    <Form
                      variant="destructive"
                      submit="Confirm removal"
                      action={async () => {
                        await changeMember(member, null);
                        return "Membership removed.";
                      }}
                    >
                      <p>Remove {member.display_name} from this workspace?</p>
                      <Button
                        variant="outline"
                        type="button"
                        onClick={() => setRemoving(null)}
                      >
                        Cancel
                      </Button>
                    </Form>
                  ) : (
                    <Button
                      variant="outline"
                      type="button"
                      onClick={() => setRemoving(member.id)}
                    >
                      Remove
                    </Button>
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
      </section>
      {manage && (
        <section className="settings-section">
          <Separator />
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
            <RoleField label="Invitation role" owner={owner} />
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
              <UIField>
                <FieldLabel htmlFor="invitation-link">
                  Single-use invitation link
                </FieldLabel>
                <Input
                  id="invitation-link"
                  readOnly
                  value={link.url}
                  onFocus={(event) => event.currentTarget.select()}
                />
              </UIField>
              <Button
                variant="outline"
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
              </Button>
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
                      inline
                      variant="outline"
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
      <section className="settings-section" id="teams">
        <Separator />
        <h2>Teams</h2>
        {detail.teams.map((team) => (
          <div className="team" key={team.id}>
            <div className="section-heading">
              <h3>
                {team.name} <Badge variant="outline">{team.key}</Badge>
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
              <FieldGroup className="grid grid-cols-1 gap-5 sm:grid-cols-2">
                <Field name="name" label="Team name" />
                <Field
                  name="key"
                  label="Team key"
                  hint="2–10 letters or numbers, e.g. ENG."
                />
              </FieldGroup>
            </Form>
          </>
        )}
      </section>
    </>
  );
}
