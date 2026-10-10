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
    <section className="issue-group" aria-label={`${team.key} ${status.name}`}>
      <div className="issue-group-heading">
        <button
          type="button"
          className="issue-group-toggle"
          onClick={() => setCollapsed((value) => !value)}
          aria-expanded={!collapsed}
        >
          {collapsed ? <ChevronRightIcon /> : <ChevronDownIcon />}
          <StatusIcon category={status.category} />
          <h2>{status.name}</h2>
          <span className="issue-group-count">{issues.length}</span>
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
        <ul className="issue-list">
          {issues.map((issue) => {
            const member = detail.members.find(
              (member) => member.id === issue.assignee_membership_id,
            );
            return (
              <li key={issue.id}>
                <Link
                  to="/$workspaceSlug/issue/$issueKey/$issueTitle"
                  params={issueRouteParams(slug, issue)}
                >
                  <span
                    className="issue-row-priority"
                    title={`${issue.priority} priority`}
                  >
                    <PriorityIcon priority={issue.priority} />
                  </span>
                  <span className="issue-identifier">{issue.identifier}</span>
                  <span className="issue-row-status" title={status.name}>
                    <StatusIcon category={status.category} />
                  </span>
                  <strong title={issue.title}>{issue.title}</strong>
                  <span className="sr-only">
                    {status.name}, {issue.priority} priority,{' '}
                    {member?.display_name ?? 'Unassigned'}
                  </span>
                  <span
                    className="issue-row-assignee"
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
                  <time dateTime={issue.created_at} className="issue-row-date">
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
    <div className="issues-page">
      <header className="issues-header">
        <div className="issues-breadcrumb">
          <span>{selectedTeam?.name ?? 'Team'}</span>
          <ChevronRightIcon aria-hidden="true" />
          <h1>Issues</h1>
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
      <div className="issues-toolbar">
        <nav aria-label="Issue lists">
          <Link
            to="/$workspaceSlug/team/$teamSlug/all"
            params={{
              workspaceSlug: slug,
              teamSlug: teamKey.toLowerCase(),
            }}
            className={cn('issue-view', !archived && 'issue-view-active')}
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
            className={cn('issue-view', archived && 'issue-view-active')}
            aria-current={archived ? 'page' : undefined}
          >
            Archived
          </Link>
        </nav>
        <span className="issues-total">
          {issues
            ? `${issues.length} ${issues.length === 1 ? 'issue' : 'issues'}`
            : ''}
        </span>
      </div>
      <div className="issues-body">
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
          <Empty className="issue-empty">
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
