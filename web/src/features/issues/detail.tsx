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
    <section className="issue-detail-page">
      <header className="issues-header">
        <nav aria-label="Issue navigation" className="issues-breadcrumb">
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
        <div className="issue-detail-actions">
          <span className="issue-save-status" role="status">
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
      <div className="issue-detail-body">
        {archived && (
          <Notice>This issue is archived. Restore it to make changes.</Notice>
        )}
        {conflict && (
          <div className="my-5 flex flex-col gap-4">
            <Notice error>
              This issue changed while you were editing. Your draft is
              preserved. Review the current values before choosing what to keep.
            </Notice>
            <dl className="conflict-values">
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
        <div className="issue-detail-layout">
          <article className="issue-document" aria-label="Issue content">
            <h1 className="issue-title-heading" aria-label={draft.title}>
              <Textarea
                id="issue-title"
                aria-label="Title"
                className="issue-inline-title"
                value={draft.title}
                maxLength={240}
                rows={1}
                disabled={disabled}
                onChange={(event) =>
                  editing.change({ title: event.target.value })
                }
                onBlur={() => void editing.save()}
              />
            </h1>
            <RichDescription
              value={draft.description}
              disabled={disabled}
              onChange={(description) => editing.change({ description })}
              onBlur={() => void editing.save()}
            />
          </article>
          <aside className="issue-properties" aria-label="Issue properties">
            <h2>Properties</h2>
            <div className="issue-property-controls">
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
          <div className="issue-save-recovery">
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
          <div className="issue-save-recovery">
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
