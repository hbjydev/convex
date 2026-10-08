import { Link, useNavigate } from "@tanstack/react-router";
import { ArrowRightIcon } from "lucide-react";
import { api } from "@/api/client";
import { useAuth } from "@/auth";
import { Field, Form, text } from "@/components/app-form";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty";
import { FieldGroup } from "@/components/ui/field";
import { Separator } from "@/components/ui/separator";
import { WorkspaceFields } from "@/features/workspaces/workspace-fields";
import { unwrap } from "@/lib/requests";

export function Home() {
  const { account, reload } = useAuth();
  const navigate = useNavigate();
  if (!account) return null;
  return (
    <>
      <div className="page-heading">
        <h1>Your workspaces</h1>
        <Badge variant="secondary">{account.workspaces.length}</Badge>
      </div>
      <p className="intro">
        People, teams, and a shared place to organize work.
      </p>
      {account.workspaces.length > 0 && (
        <div className="workspace-list-header" aria-hidden="true">
          <span>Workspace</span>
          <span>Your role</span>
        </div>
      )}
      <ul className="workspace-list">
        {account.workspaces.map((space) => (
          <li key={space.id}>
            <Link to="/$workspaceSlug" params={{ workspaceSlug: space.slug }}>
              <Avatar>
                <AvatarFallback>
                  {space.name.slice(0, 1).toUpperCase()}
                </AvatarFallback>
              </Avatar>
              <strong>{space.name}</strong>
              <Badge variant="outline">{space.role}</Badge>
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
        <section className="settings-section" id="create-workspace">
          <Separator />
          <h2>Create another workspace</h2>
          <Form
            submit="Create workspace"
            action={async (form) => {
              const space = unwrap(
                await api.POST("/api/v1/workspaces", {
                  body: {
                    name: text(form, "name"),
                    slug: text(form, "slug"),
                    team_name: text(form, "team_name"),
                    team_key: text(form, "team_key"),
                  },
                }),
              );
              await reload();
              await navigate({
                to: "/$workspaceSlug",
                params: { workspaceSlug: space.slug },
              });
              return undefined;
            }}
          >
            <WorkspaceFields />
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
