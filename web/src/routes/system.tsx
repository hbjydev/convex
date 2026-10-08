import { createFileRoute } from "@tanstack/react-router";
import { SystemPage } from "@/system-page";
export const Route = createFileRoute("/system")({ component: SystemPage });
