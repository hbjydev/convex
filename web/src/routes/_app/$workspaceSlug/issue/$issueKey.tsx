import { createFileRoute, notFound } from '@tanstack/react-router';
import { IssueDetail } from '@/features/issues/detail';
export const Route = createFileRoute('/_app/$workspaceSlug/issue/$issueKey')({
  beforeLoad: ({ params }) => {
    if (!/^[a-z][a-z0-9]{1,9}-[1-9]\d*$/i.test(params.issueKey))
      throw notFound();
  },
  component: Page,
});
function Page() {
  const { workspaceSlug, issueKey } = Route.useParams();
  return <IssueDetail slug={workspaceSlug} issueKey={issueKey} />;
}
