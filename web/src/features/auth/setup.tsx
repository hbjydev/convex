import { useNavigate } from '@tanstack/react-router';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Credentials, Field, Form, text } from '@/components/app-form';
import { AuthCard } from '@/components/auth-card';
import { Notice } from '@/components/feedback';
import { FieldGroup } from '@/components/ui/field';
import { Separator } from '@/components/ui/separator';
import { WorkspaceFields } from '@/features/workspaces/workspace-fields';
import { unwrap } from '@/lib/requests';

export function Setup() {
  const { bootstrap, reload } = useAuth();
  const navigate = useNavigate();
  return (
    <AuthCard
      title="Set up Convex"
      description="Create your first workspace and team."
    >
      {bootstrap.setup_enabled ? (
        <Form
          submit="Create workspace"
          action={async (form) => {
            unwrap(
              await api.POST('/api/v1/auth/setup', {
                body: {
                  setup_token: text(form, 'setup_token'),
                  email: text(form, 'email'),
                  password: text(form, 'password'),
                  display_name: text(form, 'display_name'),
                  workspace_name: text(form, 'workspace_name'),
                  workspace_slug: text(form, 'workspace_slug'),
                  team_name: text(form, 'team_name'),
                  team_key: text(form, 'team_key'),
                },
              }),
            );
            const account = await reload();
            const workspace = account?.workspaces[0];
            if (workspace)
              await navigate({
                to: '/$workspaceSlug',
                params: { workspaceSlug: workspace.slug },
              });
            else await navigate({ to: '/' });
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
          <FieldGroup>
            <Separator />
            <WorkspaceFields setup />
            <FieldGroup className="grid grid-cols-1 gap-5 sm:grid-cols-2">
              <Field name="team_name" label="First team name" />
              <Field
                name="team_key"
                label="Team key"
                hint="2–10 letters or numbers, e.g. ENG."
              />
            </FieldGroup>
          </FieldGroup>
        </Form>
      ) : (
        <Notice>
          Setup is waiting for your installation operator to configure a setup
          credential and restart the server.
        </Notice>
      )}
    </AuthCard>
  );
}
