import { Link, useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Form } from '@/components/app-form';
import { AuthCard } from '@/components/auth-card';
import { LoadingState, Notice } from '@/components/feedback';
import { unwrap } from '@/lib/requests';

export function OidcContinue({
  invitationToken,
  returnTo = '/',
}: {
  invitationToken?: string;
  returnTo?: '/' | '/invite';
}) {
  const { bootstrap } = useAuth();
  if (!bootstrap.oidc_display_name) return null;
  return (
    <Form
      submitClassName="h-auto min-h-8 max-w-full whitespace-normal"
      submit={`Continue with ${bootstrap.oidc_display_name}`}
      action={async () => {
        const result = unwrap(
          await api.POST('/api/v1/auth/oidc/start', {
            body: { invitation_token: invitationToken ?? null },
          }),
        );
        sessionStorage.setItem('convex-oidc-return', returnTo);
        sessionStorage.setItem(
          'convex-oidc-invitation',
          invitationToken ? 'yes' : 'no',
        );
        window.location.assign(result.authorization_url);
        return undefined;
      }}
    >
      {null}
    </Form>
  );
}

export function OidcOutcome({ outcome }: { outcome: 'success' | 'failed' }) {
  const { account } = useAuth();
  const navigate = useNavigate();
  const successful = outcome === 'success' && !!account;
  const invited = sessionStorage.getItem('convex-oidc-invitation') === 'yes';
  const returnTo =
    sessionStorage.getItem('convex-oidc-return') === '/invite'
      ? '/invite'
      : '/';
  useEffect(() => {
    if (!successful) return;
    if (invited) sessionStorage.removeItem('convex-invitation');
    sessionStorage.removeItem('convex-oidc-invitation');
    sessionStorage.removeItem('convex-oidc-return');
    void navigate({ to: invited ? '/' : returnTo, replace: true });
  }, [successful, invited, returnTo, navigate]);
  return (
    <AuthCard title={successful ? 'Signed in.' : 'Sign-in did not complete.'}>
      {successful ? (
        <LoadingState>Opening your workspace…</LoadingState>
      ) : (
        <>
          <Notice error>
            Your sign-in was canceled, expired, or could not be verified. Start
            again. If the provider is unavailable, try again later. New accounts
            need a valid workspace invitation.
          </Notice>
          <Link
            className="after-form"
            to={invited || returnTo === '/invite' ? '/invite' : '/auth/login'}
          >
            Return and try again
          </Link>
        </>
      )}
    </AuthCard>
  );
}
