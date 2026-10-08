import { createFileRoute, redirect } from "@tanstack/react-router";
import { AppShell } from "@/components/app-shell";

export const Route = createFileRoute("/_app")({
  beforeLoad: ({ context }) => {
    const auth = context.auth.getSnapshot();
    if (auth?.bootstrap.setup_required)
      throw redirect({ to: "/setup", replace: true });
    if (!auth?.account) throw redirect({ to: "/auth/login", replace: true });
  },
  component: AppShell,
});
