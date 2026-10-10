import { createFileRoute, Outlet, useLocation } from '@tanstack/react-router';
import { WorkspaceSettings } from '@/features/workspaces/settings';

export const Route = createFileRoute('/_app/$workspaceSlug')({
  beforeLoad: ({ context, params }) => {
    const workspaces = context.auth.getSnapshot()?.account?.workspaces;
    const workspace = workspaces?.find(
      (space) => space.slug === params.workspaceSlug,
    );
    return { workspaceId: workspace?.id };
  },
  component: WorkspaceRoute,
});

function WorkspaceRoute() {
  const { pathname } = useLocation();
  const { workspaceSlug } = Route.useParams();
  return pathname.replace(/\/$/, '') === `/${workspaceSlug}` ? (
    <WorkspaceSettings />
  ) : (
    <Outlet />
  );
}
