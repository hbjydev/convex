import { createFileRoute } from "@tanstack/react-router";
import { InvitationAcceptance } from "@/features/auth/invitation";
export const Route = createFileRoute("/_auth/invite")({
  component: InvitationAcceptance,
});
