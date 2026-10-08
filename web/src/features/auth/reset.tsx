import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { useAuth } from "@/auth";
import { Field, Form, text } from "@/components/app-form";
import { Notice } from "@/components/feedback";
import { unwrap } from "@/lib/requests";
import { AuthCard } from "@/components/auth-card";

export function Reset() {
  const navigate = useNavigate();
  const [token] = useState(() => {
    const value = window.location.hash.slice(1);
    return value;
  });
  useEffect(() => {
    void navigate({ to: "/auth/reset", hash: "", replace: true });
  }, [navigate]);
  const [done, setDone] = useState(false);
  const { reload } = useAuth();
  return (
    <AuthCard title="Choose a new password" description="Enter a new password to reset your account.">
      {done ? (
        <>
          <Notice>
            Your password was reset and all sessions were signed out.
          </Notice>
          <Link to="/auth/login">Sign in</Link>
        </>
      ) : token ? (
        <Form
          submit="Reset password"
          action={async (form) => {
            const result = unwrap(
              await api.POST("/api/v1/auth/reset", {
                body: { token, password: text(form, "password") },
              }),
            );
            await reload();
            setDone(true);
            return result.message;
          }}
        >
          <Field
            name="password"
            label="New password"
            type="password"
            autoComplete="new-password"
            hint="Use 12–128 characters."
          />
        </Form>
      ) : (
        <>
          <Notice>
            Open the complete reset link from your email or installation
            operator. Reset links expire after 30 minutes.
          </Notice>
          <Link to="/auth/recovery">Get a new reset link</Link>
        </>
      )}
    </AuthCard>
  );
}
