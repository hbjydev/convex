import { useNavigate } from '@tanstack/react-router';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Credentials, Field, Form, text } from '@/components/app-form';
import { AuthCard } from '@/components/auth-card';
import { Notice } from '@/components/feedback';
import { unwrap } from '@/lib/requests';

export function Setup() {
  const { bootstrap, reload } = useAuth();
  const navigate = useNavigate();
  return (
    <AuthCard
      title="Set up Convex"
      description="Create your administrator account. You’ll create a workspace next."
    >
      {bootstrap.setup_enabled ? (
        <Form
          submit="Create admin account"
          action={async (form) => {
            unwrap(
              await api.POST('/api/v1/auth/setup', {
                body: {
                  setup_token: text(form, 'setup_token'),
                  email: text(form, 'email'),
                  password: text(form, 'password'),
                  display_name: text(form, 'display_name'),
                },
              }),
            );
            await reload();
            await navigate({ to: '/workspaces/create' });
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
