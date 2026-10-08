import {
  createRootRouteWithContext,
  type ErrorComponentProps,
  Link,
  useRouter,
  Outlet,
} from "@tanstack/react-router";
import type { AuthStore } from "@/auth";
import { LoadingState, Notice } from "@/components/feedback";
import { Button } from "@/components/ui/button";
import { errorMessage } from "@/lib/requests";
import { Toaster } from "@/components/ui/sonner";

export const Route = createRootRouteWithContext<{ auth: AuthStore }>()({
  beforeLoad: ({ context }) =>
    context.auth.ensureLoaded().then(() => undefined),

  component: () => {
    return (
      <>
        <Outlet />
        <Toaster />
      </>
    );
  },

  pendingComponent: () => (
    <div className="shell">
      <main>
        <LoadingState>Connecting to Convex…</LoadingState>
      </main>
    </div>
  ),

  errorComponent: ConnectionError,
  notFoundComponent: () => (
    <>
      <h1>Page not found</h1>
      <p>This page does not exist.</p>
      <Link to="/">Return home</Link>
    </>
  ),
});

function ConnectionError({ error }: ErrorComponentProps) {
  const router = useRouter();
  return (
    <div className="shell">
      <main>
        <Notice error>{errorMessage(error)}</Notice>
        <Button className="mt-4" onClick={() => void router.invalidate()}>
          Try again
        </Button>
      </main>
    </div>
  );
}
