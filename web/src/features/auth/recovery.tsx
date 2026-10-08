import { Link } from '@tanstack/react-router';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Field, Form, text } from '@/components/app-form';
import { unwrap } from '@/lib/requests';
import { AuthCard } from '@/components/auth-card';

export function Recovery() {
  const { bootstrap } = useAuth();
  return (
    <AuthCard
      title="Recover your account"
      description="Request a password reset link for your local account."
    >
      <p className="intro">
        {!bootstrap.mail_available &&
          'Email recovery is unavailable on this installation. Ask your installation operator for a password reset link.'}
      </p>
      {bootstrap.mail_available && (
        <Form
          submit="Request reset link"
          action={async (form) =>
            unwrap(
              await api.POST('/api/v1/auth/recovery', {
                body: { email: text(form, 'email') },
              }),
            ).message
          }
        >
          <Field name="email" label="Email" type="email" autoComplete="email" />
        </Form>
      )}
      <Link className="after-form" to="/auth/login">
        Return to sign in
      </Link>
    </AuthCard>
  );
}
