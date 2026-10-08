import { createFileRoute } from "@tanstack/react-router";
import { WorkspaceSettings } from "@/features/workspaces/settings";

export const Route = createFileRoute("/_app/workspaces/$workspaceId")({
  component: WorkspaceSettings,
});
