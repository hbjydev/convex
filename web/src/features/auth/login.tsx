import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { api } from "@/api/client";
import { useAuth } from "@/auth";
import { Credentials, Form, text } from "@/components/app-form";
import { unwrap } from "@/lib/requests";
import { AuthCard } from "@/components/auth-card";

export function Login() {
  const { reload } = useAuth();
  const navigate = useNavigate();
  const search = useSearch({ from: "/_auth/auth/login" });
  const returnTo = search.return === "invite" ? "/invite" : "/";

  return (
    <AuthCard
      title="Welcome back."
      description="Sign in to your workspace. Joining for the first time? Open the invitation link from your admin."
    >
      <Form
        submit="Sign in"
        action={async (form) => {
          unwrap(
            await api.POST("/api/v1/auth/login", {
              body: {
                email: text(form, "email"),
                password: text(form, "password"),
              },
            }),
          );
          await reload();
          await navigate({ to: returnTo });
          return undefined;
        }}
      >
        <Credentials />
      </Form>
      <Link className="after-form" to="/auth/recovery">
        Forgot your password?
      </Link>
    </AuthCard>
  );
}
