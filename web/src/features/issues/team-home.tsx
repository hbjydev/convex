import { PageTitle, SectionTitle } from '@/components/typography';
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
      <div className="mb-2 flex items-center gap-2.5 [&>h1]:mb-0">
        <PageTitle>{team.name}</PageTitle>
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
        <SectionTitle id="team-workflow">Workflow</SectionTitle>
        <ul className="mt-3 max-w-[560px] [&_li]:flex [&_li]:items-center [&_li]:gap-2 [&_li]:border-b [&_li]:py-3 [&_li]:text-sm [&_svg]:size-4 [&_svg]:text-muted-foreground">
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
