import { useEffect, useState } from 'react';
import { useSearch } from '@tanstack/react-router';
import { api } from '@/api/client';
import type { components } from '@/api/schema';
import { Field, Form, text } from '@/components/app-form';
import { Notice } from '@/components/feedback';
import { Separator } from '@/components/ui/separator';
import { unwrap } from '@/lib/requests';

type Methods = components['schemas']['Methods'];
export function LoginMethods({
  methods,
  updated,
}: {
  methods: Methods;
  updated: () => Promise<void>;
}) {
  const { method } = useSearch({ from: '/_app/account' });
  const [message] = useState(() =>
    method === 'linked'
      ? 'Single sign-on linked. Other sessions have been signed out.'
      : '',
  );
  useEffect(() => {
    sessionStorage.removeItem('convex-oidc-method');
  }, []);
  async function prove(action: 'add_local' | 'remove_local') {
    const response = unwrap(
      await api.POST('/api/v1/auth/methods/oidc/prove', { body: { action } }),
    );
    sessionStorage.setItem('convex-oidc-method', 'yes');
    window.location.assign(response.authorization_url);
    return undefined;
  }
  return (
    <section className="settings-section">
      <Separator />
      <h2>Login methods</h2>
      <p>
        Adding or removing a login method signs out your other sessions. Your
        account and workspace memberships stay the same.
      </p>
      {message && <Notice>{message}</Notice>}
      {methods.local_email && (
        <p>
          Password login: {methods.local_email}
          {!methods.local_usable && ' (disabled by installation policy)'}
        </p>
      )}
      {methods.oidc.map((identity) => (
        <div key={identity.id} className="flex min-w-0 flex-col gap-3">
          <p className="break-words">
            Single sign-on: {identity.issuer}
            {!identity.usable && ' (provider disabled or no longer configured)'}
          </p>
          {methods.local_usable ? (
            <Form
              submit="Remove single sign-on"
              variant="destructive"
              action={async (form) => {
                const result = unwrap(
                  await api.POST('/api/v1/auth/methods/oidc/remove', {
                    body: {
                      identity_id: identity.id,
                      current_password: text(form, 'current_password'),
                    },
                  }),
                );
                await updated();
                return result.message;
              }}
            >
              <Field
                name="current_password"
                label="Current password to remove single sign-on"
                type="password"
                autoComplete="current-password"
                hint="Prove your remaining password login before removing this provider."
              />
            </Form>
          ) : (
            <p>
              Keep single sign-on while it is your only permitted login method.
            </p>
          )}
        </div>
      ))}
      {methods.can_link_oidc && (
        <Form
          submit={`Link ${methods.provider_display_name}`}
          action={async (form) => {
            const result = unwrap(
              await api.POST('/api/v1/auth/methods/oidc/link', {
                body: { current_password: text(form, 'current_password') },
              }),
            );
            sessionStorage.setItem('convex-oidc-method', 'yes');
            window.location.assign(result.authorization_url);
            return undefined;
          }}
        >
          <Field
            name="current_password"
            label="Current password to link single sign-on"
            type="password"
            autoComplete="current-password"
            hint="You will also sign in again with the provider; an identity belonging to another account cannot be linked."
          />
        </Form>
      )}
      {methods.can_add_local &&
        (methods.pending_action === 'add_local' ? (
          <Form
            submit="Add password"
            action={async (form) => {
              const result = unwrap(
                await api.POST('/api/v1/auth/methods/local/add', {
                  body: {
                    email: text(form, 'email'),
                    password: text(form, 'password'),
                  },
                }),
              );
              await updated();
              return result.message;
            }}
          >
            <Field
              name="email"
              label="Email for password login"
              type="email"
              autoComplete="email"
              hint="Choose an unused local login address. Matching emails do not merge accounts."
            />
            <Field
              name="password"
              label="New password"
              type="password"
              autoComplete="new-password"
              hint="Use 12–128 characters. Finish within five minutes of provider verification."
            />
          </Form>
        ) : (
          <Form
            submit="Verify single sign-on to add a password"
            action={() => prove('add_local')}
          >
            {null}
          </Form>
        ))}
      {methods.can_remove_local &&
        (methods.pending_action === 'remove_local' ? (
          <Form
            submit="Remove password"
            variant="destructive"
            action={async () => {
              const result = unwrap(
                await api.POST('/api/v1/auth/methods/local/remove'),
              );
              await updated();
              return result.message;
            }}
          >
            <p>
              Your fresh provider verification is ready. Removing the password
              also invalidates its recovery links.
            </p>
          </Form>
        ) : (
          <Form
            submit="Verify single sign-on to remove password"
            variant="outline"
            action={() => prove('remove_local')}
          >
            {null}
          </Form>
        ))}
      {methods.local_email &&
        methods.oidc.some((identity) => identity.usable) &&
        !methods.can_remove_local && (
          <p>
            Your password must remain available for installation administrator
            recovery.
          </p>
        )}
    </section>
  );
}
