import { createFileRoute, redirect } from "@tanstack/react-router";
import { Login } from "@/features/auth/login";

export const Route = createFileRoute("/_auth/auth/login")({
  validateSearch: (search: Record<string, unknown>): { return?: "invite" } => ({
    return: search.return === "invite" ? "invite" : undefined,
  }),
  beforeLoad: ({ context, search }) => {
    const auth = context.auth.getSnapshot();
    if (auth?.bootstrap.setup_required)
      throw redirect({ to: "/setup", replace: true });
    if (auth?.account)
      throw redirect({
        to: search.return === "invite" ? "/invite" : "/",
        replace: true,
      });
  },
  component: Login,
});
