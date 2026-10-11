import { Link, useNavigate } from '@tanstack/react-router';
import { ChevronLeftIcon } from 'lucide-react';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Form, text } from '@/components/app-form';
import { PageTitle, Intro } from '@/components/typography';
import { Button } from '@/components/ui/button';
import { WorkspaceFields } from './workspace-fields';
import { firstTeam } from './first-team';
import { unwrap } from '@/lib/requests';

export function CreateWorkspace() {
  const { account, reload } = useAuth();
  const navigate = useNavigate();
  if (!account) return null;
  return (
    <div className="flex min-h-svh flex-col bg-background px-5 py-5 wrap-anywhere md:px-8 md:py-7">
      <header className="flex items-start justify-between gap-5">
        <Button asChild variant="ghost">
          <Link to={account.workspaces.length ? '/' : '/account'}>
            <ChevronLeftIcon data-icon="inline-start" />
            Back to {account.workspaces.length ? 'app' : 'account'}
          </Link>
        </Button>
        <div className="min-w-0 text-right text-[13px]">
          <p className="text-muted-foreground">Logged in as</p>
          <p className="mt-1">{account.user.display_name}</p>
        </div>
      </header>
      <main
        id="main-content"
        className="flex flex-1 items-center justify-center py-12"
      >
        <div className="w-full max-w-[400px]">
          <PageTitle className="text-center">Create a workspace</PageTitle>
          <Intro className="text-center">
            A shared place for your team’s work.
          </Intro>
          <Form
            className="mt-8"
            submit="Create workspace"
            submitClassName="mt-4 w-full min-h-11"
            action={async (form) => {
              const name = text(form, 'name').trim();
              const team = firstTeam(name);
              const space = unwrap(
                await api.POST('/api/v1/workspaces', {
                  body: {
                    name,
                    slug: text(form, 'slug'),
                    team_name: team.name,
                    team_key: team.key,
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
            <WorkspaceFields focusName />
          </Form>
        </div>
      </main>
    </div>
  );
}
