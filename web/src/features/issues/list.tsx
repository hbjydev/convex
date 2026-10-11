import { SectionTitle, PageTitle } from '@/components/typography';
import { Link } from '@tanstack/react-router';
import {
  ChevronDownIcon,
  ChevronRightIcon,
  PlusIcon,
  UserRoundIcon,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { api } from '@/api/client';
import { LoadingState, Notice } from '@/components/feedback';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from '@/components/ui/empty';
import { errorMessage, unwrap } from '@/lib/requests';
import { cn } from '@/lib/utils';
import { CreateIssueDialog } from './create';
import { PriorityIcon, StatusIcon } from './icons';
import type { Issue, WorkspaceDetail } from './model';
import { issueRouteParams } from './model';
import { useIssueWorkspace } from './use-workspace';
function IssueGroup({
  issues,
  team,
  status,
  detail,
  slug,
  onCreate,
}: {
  issues: Issue[];
  team: WorkspaceDetail['teams'][number];
  status: WorkspaceDetail['teams'][number]['statuses'][number];
  detail: WorkspaceDetail;
  slug: string;
  onCreate: () => void;
}) {
  const [collapsed, setCollapsed] = useState(false);
  return (
    <section className="mb-3" aria-label={`${team.key} ${status.name}`}>
      <div className="flex items-center justify-between rounded-[5px] bg-muted px-2 py-0.5">
        <button
          type="button"
          className="flex flex-1 cursor-pointer items-center gap-[9px] py-1.5 text-left [&_svg]:size-3.5 [&_svg]:text-muted-foreground [&_[data-status-category=completed]]:text-primary"
          onClick={() => setCollapsed((value) => !value)}
          aria-expanded={!collapsed}
        >
          {collapsed ? <ChevronRightIcon /> : <ChevronDownIcon />}
          <StatusIcon category={status.category} />
          <SectionTitle className="m-0 text-[13px] font-medium">
            {status.name}
          </SectionTitle>
          <span className="text-xs text-muted-foreground tabular-nums">
            {issues.length}
          </span>
          {detail.teams.length > 1 && (
            <Badge variant="outline">{team.key}</Badge>
          )}
        </button>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={`Create issue in ${team.key} ${status.name}`}
          onClick={onCreate}
        >
          <PlusIcon />
        </Button>
      </div>
      {!collapsed && (
        <ul className="m-0 list-none p-0">
          {issues.map((issue) => {
            const member = detail.members.find(
              (member) => member.id === issue.assignee_membership_id,
            );
            return (
              <li key={issue.id} className="border-b">
                <Link
                  className="grid grid-cols-[16px_70px_16px_minmax(0,1fr)_24px_52px] items-center gap-3 px-5 py-3 text-[13px] text-foreground no-underline transition-colors duration-[120ms] ease-out hover:bg-muted focus-visible:rounded focus-visible:outline-2 focus-visible:outline-ring focus-visible:-outline-offset-2 motion-reduce:transition-none max-[600px]:grid-cols-[16px_65px_16px_minmax(0,1fr)_24px] max-[600px]:gap-2 max-[600px]:px-2"
                  to="/$workspaceSlug/issue/$issueKey/$issueTitle"
                  params={issueRouteParams(slug, issue)}
                >
                  <span
                    className="[&_svg]:size-4 [&_svg]:text-muted-foreground"
                    title={`${issue.priority} priority`}
                  >
                    <PriorityIcon priority={issue.priority} />
                  </span>
                  <span className="text-muted-foreground tabular-nums">
                    {issue.identifier}
                  </span>
                  <span
                    className="flex [&_svg]:size-4 [&_svg]:text-muted-foreground [&_[data-status-category=completed]]:text-primary"
                    title={status.name}
                  >
                    <StatusIcon category={status.category} />
                  </span>
                  <strong
                    className="truncate font-medium max-[600px]:col-[2/5] max-[600px]:row-start-2 max-[600px]:whitespace-normal max-[600px]:wrap-anywhere"
                    title={issue.title}
                  >
                    {issue.title}
                  </strong>
                  <span className="sr-only">
                    {status.name}, {issue.priority} priority,{' '}
                    {member?.display_name ?? 'Unassigned'}
                  </span>
                  <span
                    className="[&>svg]:size-4 [&>svg]:text-muted-foreground max-[600px]:col-start-5 max-[600px]:row-[1/3]"
                    title={member?.display_name ?? 'Unassigned'}
                  >
                    {member ? (
                      <Avatar size="sm">
                        <AvatarFallback>
                          {member.display_name.slice(0, 2).toUpperCase()}
                        </AvatarFallback>
                      </Avatar>
                    ) : (
                      <UserRoundIcon />
                    )}
                  </span>
                  <time
                    dateTime={issue.created_at}
                    className="text-right text-xs text-muted-foreground max-[600px]:hidden"
                  >
                    {new Intl.DateTimeFormat(undefined, {
                      month: 'short',
                      day: 'numeric',
                    }).format(new Date(issue.created_at))}
                  </time>
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
export function IssueList({
  slug,
  archived,
  initialCreate = false,
  teamKey,
}: {
  slug: string;
  archived: boolean;
  initialCreate?: boolean;
  teamKey: string;
}) {
  const workspace = useIssueWorkspace(slug);
  const [issues, setIssues] = useState<Issue[]>();
  const [error, setError] = useState('');
  const [createOpen, setCreateOpen] = useState(initialCreate);
  const [createStatus, setCreateStatus] = useState<string>();
  const [revision, setRevision] = useState(0);
  const detail = workspace.detail;
  const selectedTeam = detail?.teams.find((team) => team.key === teamKey);
  const teamId = selectedTeam?.id;
  useEffect(() => {
    let active = true;
    setIssues(undefined);
    setError('');
    if (!detail || !teamId) {
      setIssues(undefined);
      return;
    }
    api
      .GET('/api/v1/workspaces/{workspace_id}/issues', {
        params: {
          path: { workspace_id: detail.workspace.id },
          query: { archived, team_id: teamId },
        },
      })
      .then(unwrap)
      .then((value) => {
        if (active) setIssues(value);
      })
      .catch((reason) => {
        if (active) setError(errorMessage(reason));
      });
    return () => {
      active = false;
    };
    // Successful creation refreshes the collection while the dialog stays mounted.
    // oxlint-disable-next-line react/exhaustive-effect-dependencies
  }, [detail, archived, teamId, revision]);
  function openCreate(statusId?: string) {
    setCreateStatus(statusId);
    setCreateOpen(true);
  }
  const groups =
    detail?.teams
      .filter((team) => team.id === teamId)
      .flatMap((team) =>
        team.statuses.map((status) => ({
          team,
          status,
          issues:
            issues?.filter(
              (issue) =>
                issue.team_id === team.id && issue.status_id === status.id,
            ) ?? [],
        })),
      )
      .filter((group) => group.issues.length > 0) ?? [];
  return (
    <div data-layout="issues">
      <header className="flex items-center justify-between gap-4 border-b px-6 py-4 max-[600px]:flex-wrap max-[600px]:px-4 max-[600px]:py-3 in-[[data-layout=issue-detail]]:max-md:flex-wrap in-[[data-layout=issue-detail]]:max-md:gap-3">
        <div className="flex min-w-0 items-center gap-2.5 text-[13px] [&>span]:max-w-[180px] [&>span]:truncate [&>span]:text-muted-foreground max-[600px]:[&>span]:max-w-[120px] [&>svg]:size-3.5 [&>svg]:shrink-0 [&>svg]:text-muted-foreground [&_h1]:m-0 [&_h1]:text-sm [&_h1]:font-medium [&_h1]:tracking-normal [&_a]:text-primary [&_a]:underline">
          <span>{selectedTeam?.name ?? 'Team'}</span>
          <ChevronRightIcon aria-hidden="true" />
          <PageTitle>Issues</PageTitle>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={!selectedTeam}
          onClick={() => openCreate()}
        >
          <PlusIcon data-icon="inline-start" />
          New issue
        </Button>
      </header>
      <div className="flex items-center justify-between gap-4 px-6 py-2.5 max-[600px]:px-4">
        <nav className="flex gap-1" aria-label="Issue lists">
          <Link
            to="/$workspaceSlug/team/$teamSlug/all"
            params={{
              workspaceSlug: slug,
              teamSlug: teamKey.toLowerCase(),
            }}
            className={cn(
              'rounded-full px-3 py-[5px] text-xs font-medium text-muted-foreground no-underline hover:bg-muted',
              !archived && 'bg-secondary text-foreground',
            )}
            aria-current={!archived ? 'page' : undefined}
          >
            All issues
          </Link>
          <Link
            to="/$workspaceSlug/team/$teamSlug/archive"
            params={{
              workspaceSlug: slug,
              teamSlug: teamKey.toLowerCase(),
            }}
            className={cn(
              'rounded-full px-3 py-[5px] text-xs font-medium text-muted-foreground no-underline hover:bg-muted',
              archived && 'bg-secondary text-foreground',
            )}
            aria-current={archived ? 'page' : undefined}
          >
            Archived
          </Link>
        </nav>
        <span className="text-xs text-muted-foreground">
          {issues
            ? `${issues.length} ${issues.length === 1 ? 'issue' : 'issues'}`
            : ''}
        </span>
      </div>
      <div className="px-3 pb-8">
        {detail && !selectedTeam ? (
          <Notice error>
            This team is unavailable. Choose a team from the sidebar.
          </Notice>
        ) : error || workspace.error ? (
          <div className="flex flex-col gap-4">
            <Notice error>{error || workspace.error}</Notice>
            <Button
              className="w-fit"
              variant="outline"
              onClick={workspace.retry}
            >
              Try again
            </Button>
          </div>
        ) : !issues || !detail ? (
          <LoadingState>Loading issues…</LoadingState>
        ) : issues.length === 0 ? (
          <Empty className="px-6 py-25">
            <EmptyHeader>
              <EmptyTitle>
                {archived ? 'No archived issues' : 'Capture your first issue'}
              </EmptyTitle>
              <EmptyDescription>
                {archived
                  ? 'Archived work stays accessible here. Open an issue to restore it.'
                  : 'Give your team a clear next step. Start with a title and add the details as you go.'}
              </EmptyDescription>
            </EmptyHeader>
            {!archived && (
              <Button onClick={() => openCreate()}>
                <PlusIcon data-icon="inline-start" />
                Create an issue
              </Button>
            )}
          </Empty>
        ) : (
          groups.map((group) => (
            <IssueGroup
              key={group.status.id}
              {...group}
              detail={detail}
              slug={slug}
              onCreate={() => openCreate(group.status.id)}
            />
          ))
        )}
      </div>
      {detail && selectedTeam && (
        <CreateIssueDialog
          key={`${detail.workspace.id}:${selectedTeam.id}:${createStatus ?? 'default'}`}
          detail={detail}
          slug={slug}
          open={createOpen}
          onOpenChange={setCreateOpen}
          initialStatusId={createStatus}
          initialTeamId={selectedTeam.id}
          onCreated={() => setRevision((value) => value + 1)}
        />
      )}
    </div>
  );
}
