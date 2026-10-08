import { AuthShell } from "@/components/auth-shell";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_auth")({
  component: AuthShell,
});
