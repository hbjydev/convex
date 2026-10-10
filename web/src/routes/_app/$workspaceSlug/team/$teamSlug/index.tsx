import { createFileRoute } from '@tanstack/react-router';
import { TeamHome } from '@/features/issues/team-home';
export const Route = createFileRoute('/_app/$workspaceSlug/team/$teamSlug/')({
  component: Page,
});
function Page() {
  const { workspaceSlug, teamSlug } = Route.useParams();
  return <TeamHome slug={workspaceSlug} teamKey={teamSlug.toUpperCase()} />;
}
