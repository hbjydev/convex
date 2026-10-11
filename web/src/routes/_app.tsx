import { createFileRoute, redirect } from '@tanstack/react-router';
import { AppShell } from '@/components/app-shell';
import { requiresWorkspaceCreation } from '@/features/workspaces/onboarding';

export const Route = createFileRoute('/_app')({
  beforeLoad: ({ context, location }) => {
    const auth = context.auth.getSnapshot();
    if (auth?.bootstrap.setup_required)
      throw redirect({ to: '/setup', replace: true });
    if (!auth?.account) throw redirect({ to: '/auth/login', replace: true });
    if (
      requiresWorkspaceCreation(auth.account) &&
      location.pathname !== '/workspaces/create'
    )
      throw redirect({ to: '/workspaces/create', replace: true });
  },
  component: AppShell,
});
