import { createFileRoute } from "@tanstack/react-router";
import { Home } from "@/features/workspaces/home";

export const Route = createFileRoute("/_app/")({
  component: Home,
});
