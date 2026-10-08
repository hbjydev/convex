import { Link, useNavigate } from "@tanstack/react-router";
import { ArrowRightIcon } from "lucide-react";
import { api } from "@/api/client";
import { useAuth } from "@/auth";
import { Field, Form, text } from "@/components/app-form";
import { Badge } from "@/components/ui/badge";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty";
import { FieldGroup } from "@/components/ui/field";
import { Separator } from "@/components/ui/separator";
import { unwrap } from "@/lib/requests";

export function Home() {
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
            <Link
              to="/workspaces/$workspaceId"
              params={{ workspaceId: space.id }}
            >
              <strong>{space.name}</strong>
              <Badge variant="secondary">{space.role}</Badge>
              <ArrowRightIcon
                className="size-4 shrink-0 text-muted-foreground"
                aria-hidden="true"
              />
            </Link>
          </li>
        ))}
      </ul>
      {account.workspaces.length === 0 && (
        <Empty className="mt-6 border">
          <EmptyHeader>
            <EmptyTitle>No workspaces yet</EmptyTitle>
            <EmptyDescription>
              You have no active workspace memberships. Ask a workspace admin
              for an invitation.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      )}
      {account.user.is_instance_admin && (
        <section className="settings-section">
          <Separator />
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
              await navigate({
                to: "/workspaces/$workspaceId",
                params: { workspaceId: space.id },
              });
              return undefined;
            }}
          >
            <Field name="name" label="Workspace name" />
            <FieldGroup className="grid grid-cols-1 gap-5 sm:grid-cols-2">
              <Field name="team_name" label="First team name" />
              <Field name="team_key" label="Team key" />
            </FieldGroup>
          </Form>
        </section>
      )}
    </>
  );
}
