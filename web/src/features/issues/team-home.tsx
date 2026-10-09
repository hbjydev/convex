import { Link } from '@tanstack/react-router';
import { ListTodoIcon } from 'lucide-react';
import { LoadingState, Notice } from '@/components/feedback';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { StatusIcon } from './icons';
import { useIssueWorkspace } from './use-workspace';

export function TeamHome({ slug, teamKey }: { slug: string; teamKey: string }) {
  const { detail, error, retry } = useIssueWorkspace(slug);
  const team = detail?.teams.find((team) => team.key === teamKey);
  if (error)
    return (
      <div className="flex flex-col gap-4">
        <Notice error>{error}</Notice>
        <Button className="w-fit" variant="outline" onClick={retry}>
          Try again
        </Button>
      </div>
    );
  if (!detail) return <LoadingState>Loading team…</LoadingState>;
  if (!team)
    return (
      <Notice error>
        This team is unavailable. Choose a team from the sidebar.
      </Notice>
    );
  return (
    <>
      <div className="page-heading">
        <h1>{team.name}</h1>
        <Badge variant="outline">{team.key}</Badge>
      </div>
      <p className="text-sm text-muted-foreground">
        A team in {detail.workspace.name}.
      </p>
      <Button asChild className="mt-6">
        <Link
          to="/$workspaceSlug/team/$teamSlug/all"
          params={{ workspaceSlug: slug, teamSlug: team.key.toLowerCase() }}
        >
          <ListTodoIcon data-icon="inline-start" />
          Open issues
        </Link>
      </Button>
      <section className="mt-8" aria-labelledby="team-workflow">
        <h2 id="team-workflow">Workflow</h2>
        <ul className="team-workflow">
          {team.statuses.map((status) => (
            <li key={status.id}>
              <StatusIcon category={status.category} />
              <span>{status.name}</span>
            </li>
          ))}
        </ul>
      </section>
    </>
  );
}
