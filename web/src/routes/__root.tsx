import { PageTitle } from '@/components/typography';
import {
  createRootRouteWithContext,
  type ErrorComponentProps,
  Link,
  useRouter,
  Outlet,
} from '@tanstack/react-router';
import type { AuthStore } from '@/auth';
import { LoadingState, Notice } from '@/components/feedback';
import { Button } from '@/components/ui/button';
import { errorMessage } from '@/lib/requests';
import { Toaster } from '@/components/ui/sonner';

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
    <div className="mx-auto flex min-h-svh max-w-[1100px] flex-col px-10 wrap-anywhere max-[600px]:px-5">
      <main className="w-full min-w-0 flex-1 pt-12 pb-16 max-[600px]:pt-9 max-[600px]:pb-12">
        <LoadingState>Connecting to Convex…</LoadingState>
      </main>
    </div>
  ),

  errorComponent: ConnectionError,
  notFoundComponent: () => (
    <>
      <PageTitle>Page not found</PageTitle>
      <p>This page does not exist.</p>
      <Link to="/">Return home</Link>
    </>
  ),
});

function ConnectionError({ error }: ErrorComponentProps) {
  const router = useRouter();
  return (
    <div className="mx-auto flex min-h-svh max-w-[1100px] flex-col px-10 wrap-anywhere max-[600px]:px-5">
      <main className="w-full min-w-0 flex-1 pt-12 pb-16 max-[600px]:pt-9 max-[600px]:pb-12">
        <Notice error>{errorMessage(error)}</Notice>
        <Button className="mt-4" onClick={() => void router.invalidate()}>
          Try again
        </Button>
      </main>
    </div>
  );
}
