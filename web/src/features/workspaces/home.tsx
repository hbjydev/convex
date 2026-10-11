import { PageTitle, Intro, SectionTitle } from '@/components/typography';
import { SettingsSection } from '@/components/settings-layout';
import { Link, useNavigate } from '@tanstack/react-router';
import { ArrowRightIcon } from 'lucide-react';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Field, Form, text } from '@/components/app-form';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from '@/components/ui/empty';
import { FieldGroup } from '@/components/ui/field';
import { Separator } from '@/components/ui/separator';
import { WorkspaceFields } from '@/features/workspaces/workspace-fields';
import { unwrap } from '@/lib/requests';

export function Home() {
  const { account, reload } = useAuth();
  const navigate = useNavigate();
  if (!account) return null;
  return (
    <>
      <div className="mb-2 flex items-center gap-2.5 [&>h1]:mb-0">
        <PageTitle>Your workspaces</PageTitle>
        <Badge variant="secondary">{account.workspaces.length}</Badge>
      </div>
      <Intro>People, teams, and a shared place to organize work.</Intro>
      {account.workspaces.length > 0 && (
        <div
          className="mt-7 flex justify-between border-y bg-muted py-2.5 pr-11 pl-3 text-xs text-muted-foreground md:pr-12 md:pl-4"
          aria-hidden="true"
        >
          <span>Workspace</span>
          <span>Your role</span>
        </div>
      )}
      <ul className="m-0 list-none p-0">
        {account.workspaces.map((space) => (
          <li key={space.id}>
            <Link
              className="flex items-center gap-3 rounded-none border-b px-3 py-3.5 no-underline transition-colors duration-150 ease-out hover:bg-accent focus-visible:outline-2 focus-visible:outline-ring focus-visible:outline-offset-3 motion-reduce:transition-none md:px-4"
              to="/$workspaceSlug"
              params={{ workspaceSlug: space.slug }}
            >
              <Avatar>
                <AvatarFallback>
                  {space.name.slice(0, 1).toUpperCase()}
                </AvatarFallback>
              </Avatar>
              <strong className="min-w-0 flex-1 text-sm font-medium">
                {space.name}
              </strong>
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
        <SettingsSection id="create-workspace">
          <Separator />
          <SectionTitle>Create another workspace</SectionTitle>
          <Form
            submit="Create workspace"
            action={async (form) => {
              const space = unwrap(
                await api.POST('/api/v1/workspaces', {
                  body: {
                    name: text(form, 'name'),
                    slug: text(form, 'slug'),
                    team_name: text(form, 'team_name'),
                    team_key: text(form, 'team_key'),
                  },
                }),
              );
              await reload();
              await navigate({
                to: '/$workspaceSlug',
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
        </SettingsSection>
      )}
    </>
  );
}
