import { createFileRoute, notFound, redirect } from '@tanstack/react-router';

export const Route = createFileRoute('/_app/workspaces/$workspaceId')({
  beforeLoad: ({ context, params, location }) => {
    const workspaces = context.auth.getSnapshot()?.account?.workspaces;
    const workspace =
      workspaces?.find((space) => space.id === params.workspaceId) ??
      workspaces?.find((space) => space.slug === params.workspaceId);
    if (!workspace) throw notFound();
    throw redirect({
      to: '/$workspaceSlug',
      params: { workspaceSlug: workspace.slug },
      hash: location.hash,
      replace: true,
    });
  },
});
