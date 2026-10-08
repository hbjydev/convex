import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/api/client";
import type { components } from "@/api/schema";
import { useAuth } from "@/auth";
import { Credentials, Form, text } from "@/components/app-form";
import { LoadingState, Notice } from "@/components/feedback";
import { errorMessage, unwrap } from "@/lib/requests";

type Schema = components["schemas"];
export function InvitationAcceptance() {
  const { account, reload } = useAuth();
  const navigate = useNavigate();
  const [token] = useState(() => {
    const token =
      window.location.hash.slice(1) ||
      sessionStorage.getItem("convex-invitation") ||
      "";
    if (token) sessionStorage.setItem("convex-invitation", token);
    return token;
  });
  useEffect(() => {
    void navigate({ to: "/invite", hash: "", replace: true });
  }, [navigate]);
  const [preview, setPreview] = useState<Schema["InvitePreview"] | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!token) {
      setError(
        "Open the complete invitation link shared by your workspace admin.",
      );
      return;
    }
    let active = true;
    api
      .POST("/api/v1/invitations/preview", { body: { token } })
      .then(unwrap)
      .then((data) => {
        if (active) setPreview(data);
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      });
    return () => {
      active = false;
    };
  }, [token]);
  return (
    <>
      <h1>
        {preview ? `Join ${preview.workspace_name}` : "Workspace invitation"}
      </h1>
      {error ? (
        <Notice error>{error}</Notice>
      ) : preview ? (
        <>
          <p className="intro">
            You’ve been invited as a workspace {preview.role}.{" "}
            {account
              ? `Continue as ${account.user.display_name}.`
              : "Create your account to join, or sign in with your existing account."}
          </p>
          <Form
            submit="Join workspace"
            action={async (form) => {
              const space = unwrap(
                await api.POST("/api/v1/invitations/accept", {
                  body: {
                    token,
                    ...(account
                      ? {}
                      : {
                          email: text(form, "email"),
                          password: text(form, "password"),
                          display_name: text(form, "display_name"),
                        }),
                  },
                }),
              );
              sessionStorage.removeItem("convex-invitation");
              await reload();
              await navigate({
                to: "/$workspaceSlug",
                params: { workspaceSlug: space.slug },
              });
              return undefined;
            }}
          >
            {!account && <Credentials creating />}
          </Form>
          {!account && (
            <Link
              className="after-form"
              to="/auth/login"
              search={{ return: "invite" }}
            >
              Already have an account? Sign in
            </Link>
          )}
        </>
      ) : (
        <LoadingState>Checking your invitation…</LoadingState>
      )}
    </>
  );
}
