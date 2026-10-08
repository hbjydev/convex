import { Link } from '@tanstack/react-router';
import { ArrowLeftIcon } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api } from '@/api/client';
import type { components } from '@/api/schema';
import { useAuth } from '@/auth';
import { Field, Form, text } from '@/components/app-form';
import { Notice } from '@/components/feedback';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { errorMessage, unwrap } from '@/lib/requests';

type Schema = components['schemas'];

export function AccountSettings() {
  const { account, reload } = useAuth();
  const [sessions, setSessions] = useState<Schema['SessionInfo'][]>([]);
  const [accounts, setAccounts] = useState<Schema['AdminAccount'][]>([]);
  const [error, setError] = useState('');
  const load = useCallback(async () => {
    setSessions(unwrap(await api.GET('/api/v1/auth/sessions')));
    if (account?.user.is_instance_admin)
      setAccounts(unwrap(await api.GET('/api/v1/admin/accounts')));
  }, [account?.user.is_instance_admin]);
  useEffect(() => {
    load().catch((error) => setError(errorMessage(error)));
  }, [load]);
  return (
    <>
      <Button asChild variant="ghost" className="mb-6 w-fit">
        <Link to="/">
          <ArrowLeftIcon data-icon="inline-start" />
          Workspaces
        </Link>
      </Button>
      <h1>Your account</h1>
      {error && <Notice error>{error}</Notice>}
      <section className="settings-section">
        <Separator />
        <h2>Change password</h2>
        <Form
          submit="Change password"
          action={async (form) => {
            const result = unwrap(
              await api.POST('/api/v1/auth/password', {
                body: {
                  current_password: text(form, 'current_password'),
                  new_password: text(form, 'new_password'),
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
      <section className="settings-section">
        <Separator />
        <h2>Active sessions</h2>
        <ul className="invitation-list">
          {sessions.map((session) => (
            <li key={session.id}>
              <div>
                <strong>
                  {session.current ? 'This browser' : 'Another session'}
                </strong>
                <span>
                  Signed in {new Date(session.created_at).toLocaleString()}
                </span>
                <span>
                  Expires {new Date(session.expires_at).toLocaleString()}
                </span>
              </div>
              <Form
                inline
                variant="outline"
                submit="Sign out session"
                action={async () => {
                  const result = unwrap(
                    await api.POST('/api/v1/auth/sessions/revoke', {
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
        <section className="settings-section">
          <Separator />
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
                    {user.email} · {user.disabled ? 'Disabled' : 'Active'}
                  </span>
                </div>
                {user.id !== account.user.id && (
                  <Form
                    inline
                    variant={user.disabled ? 'outline' : 'destructive'}
                    submit={
                      user.disabled ? 'Enable account' : 'Disable account'
                    }
                    action={async () => {
                      const result = unwrap(
                        await api.POST('/api/v1/admin/accounts/disable', {
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
