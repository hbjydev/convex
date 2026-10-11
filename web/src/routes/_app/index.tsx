import { createFileRoute, redirect } from '@tanstack/react-router';
import { preferredWorkspace } from '@/features/workspaces/use-current-workspace';

export const Route = createFileRoute('/_app/')({
  beforeLoad: ({ context }) => {
    const workspace = preferredWorkspace(
      context.auth.getSnapshot()?.account?.workspaces ?? [],
    );
    if (!workspace) throw redirect({ to: '/workspaces/create', replace: true });
    throw redirect({
      to: '/workspaces/$workspaceId',
      params: { workspaceId: workspace.id },
      replace: true,
    });
  },
});
