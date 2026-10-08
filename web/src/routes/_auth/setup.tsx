import { createFileRoute, redirect } from "@tanstack/react-router";
import { Setup } from "@/features/auth/setup";

export const Route = createFileRoute("/_auth/setup")({
  beforeLoad: ({ context }) => {
    if (!context.auth.getSnapshot()?.bootstrap.setup_required)
      throw redirect({ to: "/", replace: true });
  },
  component: Setup,
});
