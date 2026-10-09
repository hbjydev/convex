import { createFileRoute } from '@tanstack/react-router';
import { IssueList } from '@/features/issues/list';
export const Route = createFileRoute('/_app/$workspaceSlug/team/$teamSlug/new')(
  { component: Page },
);
function Page() {
  const { workspaceSlug, teamSlug } = Route.useParams();
  return (
    <IssueList
      key={teamSlug}
      slug={workspaceSlug}
      teamKey={teamSlug.toUpperCase()}
      archived={false}
      initialCreate
    />
  );
}
