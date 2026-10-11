import { PageTitle, SectionTitle } from '@/components/typography';
import {
  Link,
  useBlocker,
  useLocation,
  useNavigate,
} from '@tanstack/react-router';
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { ChevronRightIcon, UserRoundIcon } from 'lucide-react';
import { api } from '@/api/client';
import { LoadingState, Notice } from '@/components/feedback';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import { errorMessage, unwrap } from '@/lib/requests';
import { IssueDescription } from './description';
import { createIssueEditing } from './editing';
import { PriorityIcon, StatusIcon } from './icons';
import { PropertyMenu } from './property-menu';
import { RichDescription } from './rich-description';
import {
  issueRouteParams,
  priorities,
  type Issue,
  type WorkspaceDetail,
} from './model';
import { useIssueWorkspace } from './use-workspace';

function Editor({
  initial,
  detail,
  onCurrentIssue,
}: {
  initial: Issue;
  detail: WorkspaceDetail;
  onCurrentIssue: (issue: Issue) => void;
}) {
  const [editing] = useState(() =>
    createIssueEditing(
      initial,
      async (patch) => {
        try {
          const result = await api.PATCH(
            '/api/v1/workspaces/{workspace_id}/issues/{issue_ref}',
            {
              params: {
                path: {
                  workspace_id: initial.workspace_id,
                  issue_ref: initial.id,
                },
              },
              body: patch,
            },
          );
          if (
            result.response.status === 409 &&
            result.error &&
            'current_issue' in result.error
          )
            return { conflict: result.error.current_issue };
          return { issue: unwrap(result) };
        } catch (reason) {
          throw new Error(errorMessage(reason));
        }
      },
      onCurrentIssue,
    ),
  );
  const state = useSyncExternalStore(editing.subscribe, editing.getSnapshot);
  const { issue, draft, pending, error, conflict, review } = state;
  const dirty = editing.dirty();
  const archived = !!issue.archived_at;
  const disabled = archived || !!conflict || state.archiving;
  const team = detail.teams.find((team) => team.id === issue.team_id);
  const status = team?.statuses.find((status) => status.id === draft.status_id);
  const currentVersionButton = useRef<HTMLButtonElement>(null);
  const hadConflict = useRef(false);
  useEffect(() => {
    editing.activate();
    return editing.dispose;
  }, [editing]);
  useEffect(() => {
    if (conflict) {
      hadConflict.current = true;
      currentVersionButton.current?.focus();
    } else if (hadConflict.current) {
      hadConflict.current = false;
      document.getElementById('issue-title')?.focus();
    }
  }, [conflict]);
  const propertyMenuClosed = (event: Event) => {
    if (editing.getSnapshot().conflict) {
      event.preventDefault();
      currentVersionButton.current?.focus();
    }
  };
  const blocker = useBlocker({
    withResolver: true,
    enableBeforeUnload: () => editing.dirty() || editing.getSnapshot().pending,
    shouldBlockFn: async ({ next }) => {
      const prefix = `/${detail.workspace.slug}/issue/${issue.identifier}`;
      if (next.pathname === prefix || next.pathname.startsWith(`${prefix}/`))
        return false;
      await editing.save();
      return editing.dirty() || editing.getSnapshot().pending;
    },
  });
  return (
    <section data-layout="issue-detail">
      <header className="flex items-center justify-between gap-4 border-b px-6 py-4 max-[600px]:flex-wrap max-[600px]:px-4 max-[600px]:py-3 in-[[data-layout=issue-detail]]:max-md:flex-wrap in-[[data-layout=issue-detail]]:max-md:gap-3">
        <nav
          aria-label="Issue navigation"
          className="flex min-w-0 items-center gap-2.5 text-[13px] [&>span]:max-w-[180px] [&>span]:truncate [&>span]:text-muted-foreground max-[600px]:[&>span]:max-w-[120px] [&>svg]:size-3.5 [&>svg]:shrink-0 [&>svg]:text-muted-foreground [&_h1]:m-0 [&_h1]:text-sm [&_h1]:font-medium [&_h1]:tracking-normal [&_a]:text-primary [&_a]:underline"
        >
          <span>{team?.name}</span>
          <ChevronRightIcon aria-hidden="true" />
          <Link
            aria-label="Back to issues"
            to="/$workspaceSlug/team/$teamSlug/all"
            params={{
              workspaceSlug: detail.workspace.slug,
              teamSlug: team?.key.toLowerCase() ?? '',
            }}
          >
            Issues
          </Link>
          <ChevronRightIcon aria-hidden="true" />
          <span>{issue.identifier}</span>
        </nav>
        <div className="flex flex-wrap items-center gap-2 max-md:[&_[data-slot=button]]:min-h-11">
          <span className="text-xs text-muted-foreground" role="status">
            {pending
              ? 'Saving…'
              : conflict
                ? 'Review conflict'
                : error
                  ? 'Not saved'
                  : review
                    ? 'Review changes'
                    : dirty
                      ? 'Unsaved changes'
                      : 'Saved'}
          </span>
          <Button
            size="sm"
            variant="ghost"
            disabled={pending || !!conflict || review || !!error}
            onClick={() => void editing.archive()}
          >
            {archived ? 'Restore issue' : 'Archive issue'}
          </Button>
        </div>
      </header>
      <div className="mx-auto max-w-[1200px] px-4 py-6 md:px-6 md:py-8 lg:p-12 [&>[role=status]]:my-4 [&>[role=alert]]:my-4">
        {archived && (
          <Notice>This issue is archived. Restore it to make changes.</Notice>
        )}
        {conflict && (
          <div className="my-5 flex flex-col gap-4">
            <Notice error>
              This issue changed while you were editing. Your draft is
              preserved. Review the current values before choosing what to keep.
            </Notice>
            <dl className="grid max-w-[780px] grid-cols-[minmax(100px,140px)_minmax(0,1fr)] gap-x-4 gap-y-3 text-sm [&_dt]:text-muted-foreground [&_dd]:wrap-anywhere">
              <dt>Current title</dt>
              <dd>{conflict.title}</dd>
              <dt>Current description</dt>
              <dd>
                {conflict.description ? (
                  <IssueDescription description={conflict.description} />
                ) : (
                  'No description'
                )}
              </dd>
              <dt>Current status</dt>
              <dd>
                {
                  team?.statuses.find((item) => item.id === conflict.status_id)
                    ?.name
                }
              </dd>
              <dt>Current priority</dt>
              <dd>{conflict.priority}</dd>
              <dt>Current assignee</dt>
              <dd>
                {detail.members.find(
                  (member) => member.id === conflict.assignee_membership_id,
                )?.display_name ?? 'Unassigned'}
              </dd>
              <dt>Archived</dt>
              <dd>{conflict.archived_at ? 'Yes' : 'No'}</dd>
            </dl>
            <div className="flex flex-wrap gap-2">
              <Button
                ref={currentVersionButton}
                variant="outline"
                onClick={() => editing.resolve(false)}
              >
                Use current version
              </Button>
              <Button
                variant="outline"
                disabled={!!conflict.archived_at}
                onClick={() => editing.resolve(true)}
              >
                Keep my changed fields for review
              </Button>
            </div>
          </div>
        )}
        <div className="grid grid-cols-[minmax(0,1fr)] items-start gap-8 md:grid-cols-[minmax(0,1fr)_200px] lg:grid-cols-[minmax(0,1fr)_240px] lg:gap-14">
          <article className="min-w-0" aria-label="Issue content">
            <PageTitle
              className="mb-6 text-[28px] leading-[1.35] tracking-[-0.025em] text-pretty wrap-anywhere md:text-2xl"
              aria-label={draft.title}
            >
              <Textarea
                className="rounded-none!"
                id="issue-title"
                aria-label="Title"
                variant="inline"
                value={draft.title}
                maxLength={240}
                rows={1}
                disabled={disabled}
                onChange={(event) =>
                  editing.change({ title: event.target.value })
                }
                onBlur={() => void editing.save()}
              />
            </PageTitle>
            <RichDescription
              value={draft.description}
              disabled={disabled}
              onChange={(description) => editing.change({ description })}
              onBlur={() => void editing.save()}
            />
          </article>
          <aside
            className="border-t pt-6 md:border-0 md:pt-0 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:text-muted-foreground"
            aria-label="Issue properties"
          >
            <SectionTitle className="mb-3 text-[13px] font-medium text-muted-foreground">
              Properties
            </SectionTitle>
            <div className="flex flex-col items-stretch gap-1">
              <PropertyMenu
                onCloseAutoFocus={propertyMenuClosed}
                inline
                disabled={disabled}
                label="Status"
                value={draft.status_id}
                icon={<StatusIcon category={status?.category} />}
                options={
                  team?.statuses.map((item) => ({
                    value: item.id,
                    label: item.name,
                  })) ?? []
                }
                onChange={(status_id) => editing.change({ status_id }, true)}
              />
              <PropertyMenu
                onCloseAutoFocus={propertyMenuClosed}
                inline
                disabled={disabled}
                label="Priority"
                value={draft.priority}
                icon={<PriorityIcon priority={draft.priority} />}
                options={priorities.map((value) => ({
                  value,
                  label:
                    value === 'none'
                      ? 'No priority'
                      : value.charAt(0).toUpperCase() + value.slice(1),
                }))}
                onChange={(value) => {
                  const priority = priorities.find((item) => item === value);
                  if (priority) editing.change({ priority }, true);
                }}
              />
              <PropertyMenu
                onCloseAutoFocus={propertyMenuClosed}
                inline
                disabled={disabled}
                label="Assignee"
                value={draft.assignee_membership_id ?? ''}
                icon={<UserRoundIcon data-icon="inline-start" />}
                options={[
                  { value: '', label: 'Unassigned' },
                  ...detail.members
                    .filter((member) => !member.disabled)
                    .map((member) => ({
                      value: member.id,
                      label: member.display_name,
                    })),
                ]}
                onChange={(value) =>
                  editing.change(
                    { assignee_membership_id: value || null },
                    true,
                  )
                }
              />
            </div>
          </aside>
        </div>
        {error && (
          <div className="mt-6 flex flex-wrap items-center gap-3 [&>[role=alert]]:basis-full">
            <Notice error>{error} Your draft is preserved.</Notice>
            <Button variant="outline" onClick={() => void editing.save(true)}>
              Retry save
            </Button>
            <Button variant="ghost" onClick={editing.discard}>
              Discard changes
            </Button>
          </div>
        )}
        {review && (
          <div className="mt-6 flex flex-wrap items-center gap-3 [&>[role=alert]]:basis-full">
            <p className="text-sm text-muted-foreground">
              Your changes are ready for review. Check them, then save.
            </p>
            <Button disabled={pending} onClick={() => void editing.save(true)}>
              Save reviewed changes
            </Button>
            <Button
              disabled={pending}
              variant="ghost"
              onClick={editing.discard}
            >
              Discard changes
            </Button>
          </div>
        )}
      </div>
      <Dialog
        open={blocker.status === 'blocked'}
        onOpenChange={(open) => {
          if (!open) blocker.reset?.();
        }}
      >
        <DialogContent>
          <DialogTitle>Unsaved changes</DialogTitle>
          <DialogDescription>
            Your changes could not be saved. Stay on this issue to retry, or
            discard them before leaving.
          </DialogDescription>
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => blocker.reset?.()}>Stay on issue</Button>
            <Button
              variant="outline"
              onClick={() => {
                editing.discard();
                blocker.proceed?.();
              }}
            >
              Discard and leave
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </section>
  );
}
export function IssueDetail({
  slug,
  issueKey,
}: {
  slug: string;
  issueKey: string;
}) {
  const workspace = useIssueWorkspace(slug);
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const [issue, setIssue] = useState<Issue>();
  const [error, setError] = useState('');
  const detail = workspace.detail;
  useEffect(() => {
    let active = true;
    setIssue(undefined);
    setError('');
    if (!detail) return;
    api
      .GET('/api/v1/workspaces/{workspace_id}/issues/{issue_ref}', {
        params: {
          path: {
            workspace_id: detail.workspace.id,
            issue_ref: issueKey.toUpperCase(),
          },
        },
      })
      .then(unwrap)
      .then((value) => {
        if (active) {
          setIssue(value);
        }
      })
      .catch((reason) => {
        if (active) setError(errorMessage(reason));
      });
    return () => {
      active = false;
    };
  }, [detail, issueKey]);
  useEffect(() => {
    if (!issue) return;
    const prefix = `/${slug}/issue/`;
    const urlKey = pathname.startsWith(prefix)
      ? pathname.slice(prefix.length).split('/')[0]?.toUpperCase()
      : undefined;
    // Router transitions can update the location before this page unmounts.
    // Only normalize the open issue, never a navigation away from it.
    if (urlKey !== issueKey.toUpperCase() || issue.identifier !== urlKey)
      return;
    const params = issueRouteParams(slug, issue);
    const canonicalPath = `/${slug}/issue/${params.issueKey}/${params.issueTitle}`;
    if (decodeURI(pathname) !== canonicalPath)
      void navigate({
        to: '/$workspaceSlug/issue/$issueKey/$issueTitle',
        params,
        replace: true,
      });
  }, [issue, slug, pathname, issueKey, navigate]);
  const owningTeam = detail?.teams.find((team) => team.id === issue?.team_id);
  return (
    <>
      {owningTeam && (error || workspace.error) && (
        <Link
          to="/$workspaceSlug/team/$teamSlug/all"
          params={{
            workspaceSlug: slug,
            teamSlug: owningTeam.key.toLowerCase(),
          }}
        >
          Back to issues
        </Link>
      )}
      {error || workspace.error ? (
        <div className="mt-6 flex flex-col gap-4">
          <Notice error>{error || workspace.error}</Notice>
          <Button
            className="w-fit"
            variant="outline"
            onClick={() => {
              workspace.retry();
            }}
          >
            Try again
          </Button>
        </div>
      ) : issue && detail ? (
        <Editor
          key={issue.id}
          initial={issue}
          detail={detail}
          onCurrentIssue={setIssue}
        />
      ) : (
        <LoadingState>Loading issue…</LoadingState>
      )}
    </>
  );
}
