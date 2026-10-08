import { createFileRoute } from "@tanstack/react-router";
import { Recovery } from "@/features/auth/recovery";
export const Route = createFileRoute("/_auth/auth/recovery")({ component: Recovery });
