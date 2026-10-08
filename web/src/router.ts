import { createRouter } from "@tanstack/react-router";
import { createAuthStore } from "@/auth";
import { routeTree } from "@/routeTree.gen";

export const router = createRouter({
  routeTree,
  context: { auth: createAuthStore() },
  scrollRestoration: true,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
