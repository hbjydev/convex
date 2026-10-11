import { createFileRoute, notFound, redirect } from '@tanstack/react-router';
import { api } from '@/api/client';
import { unwrap } from '@/lib/requests';

export const Route = createFileRoute('/_app/workspaces/$workspaceId')({
  beforeLoad: async ({ context, params, location }) => {
    const workspaces = context.auth.getSnapshot()?.account?.workspaces;
    const workspace =
      workspaces?.find((space) => space.id === params.workspaceId) ??
      workspaces?.find((space) => space.slug === params.workspaceId);
    if (!workspace) throw notFound();
    if (!location.hash) {
      const detail = unwrap(
        await api.GET('/api/v1/workspaces/{workspace_id}', {
          params: { path: { workspace_id: workspace.id } },
        }),
      );
      const team = detail.teams[0];
      if (team)
        throw redirect({
          to: '/$workspaceSlug/team/$teamSlug',
          params: {
            workspaceSlug: workspace.slug,
            teamSlug: team.key.toLowerCase(),
          },
          replace: true,
        });
    }
    throw redirect({
      to: '/$workspaceSlug',
      params: { workspaceSlug: workspace.slug },
      hash: location.hash,
      replace: true,
    });
  },
});
