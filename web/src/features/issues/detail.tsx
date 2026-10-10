import { Link, useLocation, useNavigate } from '@tanstack/react-router';
import { useEffect, useRef, useState } from 'react';
import { api } from '@/api/client';
import { LoadingState, Notice } from '@/components/feedback';
import { ChevronRightIcon, PencilIcon, UserRoundIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { FieldSet } from '@/components/ui/field';
import { Spinner } from '@/components/ui/spinner';
import { errorMessage, unwrap } from '@/lib/requests';
import { IssueFields } from './fields';
import { IssueDescription } from './description';
import { PriorityIcon, StatusIcon } from './icons';
import {
  issuePatch,
  issueRouteParams,
  type Draft,
  type Issue,
  type IssuePatch,
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
  const [issue, setIssue] = useState(initial);
  const [draft, setDraft] = useState<Draft>(initial);
  const [conflict, setConflict] = useState<Issue>();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [editing, setEditing] = useState(false);
  const editButton = useRef<HTMLButtonElement>(null);
  const wasEditing = useRef(false);
  useEffect(() => {
    if (editing) {
      wasEditing.current = true;
      document.getElementById('issue-title')?.focus();
    } else if (wasEditing.current) {
      wasEditing.current = false;
      editButton.current?.focus();
    }
  }, [editing]);
  const currentVersionButton = useRef<HTMLButtonElement>(null);
  const hadConflict = useRef(false);
  useEffect(() => {
    if (conflict) {
      hadConflict.current = true;
      currentVersionButton.current?.focus();
    } else if (hadConflict.current) {
      hadConflict.current = false;
      document.getElementById('issue-title')?.focus();
    }
  }, [conflict]);
  async function save(patch: IssuePatch) {
    setPending(true);
    setError('');
    setMessage('');
    try {
      const result = await api.PATCH(
        '/api/v1/workspaces/{workspace_id}/issues/{issue_ref}',
        {
          params: {
            path: { workspace_id: issue.workspace_id, issue_ref: issue.id },
          },
          body: patch,
        },
      );
      if (
        result.response.status === 409 &&
        result.error &&
        'current_issue' in result.error
      ) {
        setConflict(result.error.current_issue);
        return;
      }
      const saved = unwrap(result);
      setIssue(saved);
      onCurrentIssue(saved);
      setDraft(saved);
      setConflict(undefined);
      if (saved.archived_at) setEditing(false);
      setMessage(
        saved.archived_at
          ? 'Issue archived.'
          : patch.archived === false
            ? 'Issue restored.'
            : 'Changes saved.',
      );
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setPending(false);
    }
  }
  const archived = !!issue.archived_at;
  const dirty = Object.keys(issuePatch(issue, draft)).length > 1;
  const team = detail.teams.find((team) => team.id === issue.team_id);
  const status = team?.statuses.find((status) => status.id === issue.status_id);
  const assignee = detail.members.find(
    (member) => member.id === issue.assignee_membership_id,
  );
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
          <span className="issue-detail-key">{issue.identifier}</span>
        </nav>
        <div className="issue-detail-actions">
          {!archived && (
            <Button
              ref={editButton}
              variant="outline"
              size="sm"
              disabled={pending || !!conflict || (editing && dirty)}
              onClick={() => setEditing(!editing)}
            >
              <PencilIcon data-icon="inline-start" />
              {editing ? 'View issue' : 'Edit issue'}
            </Button>
          )}
          <Button
            size="sm"
            variant="ghost"
            disabled={pending || !!conflict || dirty}
            onClick={() =>
              void save({
                expected_version: issue.version,
                archived: !archived,
              })
            }
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
                  detail.teams
                    .find((team) => team.id === conflict.team_id)
                    ?.statuses.find(
                      (status) => status.id === conflict.status_id,
                    )?.name
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
                variant="outline"
                ref={currentVersionButton}
                onClick={() => {
                  setIssue(conflict);
                  onCurrentIssue(conflict);
                  setDraft(conflict);
                  setConflict(undefined);
                }}
              >
                Use current version
              </Button>
              <Button
                variant="outline"
                disabled={!!conflict.archived_at}
                onClick={() => {
                  const patch = issuePatch(issue, draft);
                  const merged: Draft = {
                    title: patch.title ?? conflict.title,
                    description: patch.description ?? conflict.description,
                    status_id: patch.status_id ?? conflict.status_id,
                    priority: patch.priority ?? conflict.priority,
                    assignee_membership_id:
                      'assignee_membership_id' in patch
                        ? (patch.assignee_membership_id ?? null)
                        : conflict.assignee_membership_id,
                  };
                  setIssue(conflict);
                  onCurrentIssue(conflict);
                  setDraft(merged);
                  setConflict(undefined);
                  setMessage(
                    'Your changes are ready for review. Check them, then save.',
                  );
                }}
              >
                Keep my changed fields for review
              </Button>
            </div>
          </div>
        )}
        {editing && !archived ? (
          <form
            className="app-form issue-editor"
            onSubmit={(event) => {
              event.preventDefault();
              void save(issuePatch(issue, draft));
            }}
            aria-busy={pending}
          >
            <FieldSet disabled={pending || archived || !!conflict}>
              <IssueFields
                draft={draft}
                onChange={setDraft}
                detail={detail}
                teamId={issue.team_id}
                disabled={pending || archived || !!conflict}
              />
              <div className="issue-editor-actions">
                <Button
                  type="submit"
                  disabled={pending || archived || !!conflict}
                >
                  {pending && <Spinner data-icon="inline-start" />}Save changes
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={pending || !!conflict}
                  onClick={() => {
                    setDraft(issue);
                    setEditing(false);
                    setError('');
                    setMessage('');
                  }}
                >
                  Cancel editing
                </Button>
                {dirty && (
                  <span className="text-sm text-muted-foreground">
                    Unsaved changes
                  </span>
                )}
              </div>
            </FieldSet>
          </form>
        ) : (
          <div className="issue-detail-layout">
            <article className="issue-document" aria-label="Issue content">
              <h1>{issue.title}</h1>
              {issue.description.trim() ? (
                <IssueDescription description={issue.description} />
              ) : (
                <p className="issue-no-description">No description yet.</p>
              )}
            </article>
            <aside className="issue-properties" aria-label="Issue properties">
              <h2>Properties</h2>
              <dl>
                <div>
                  <dt className="sr-only">Status</dt>
                  <dd>
                    <StatusIcon category={status?.category} />
                    {status?.name}
                  </dd>
                </div>
                <div>
                  <dt className="sr-only">Priority</dt>
                  <dd>
                    <PriorityIcon priority={issue.priority} />
                    {issue.priority === 'none'
                      ? 'No priority'
                      : issue.priority.charAt(0).toUpperCase() +
                        issue.priority.slice(1)}
                  </dd>
                </div>
                <div>
                  <dt className="sr-only">Assignee</dt>
                  <dd>
                    <UserRoundIcon aria-hidden="true" />
                    {assignee?.display_name ?? 'Unassigned'}
                  </dd>
                </div>
              </dl>
            </aside>
          </div>
        )}
        {error && <Notice error>{error}</Notice>}
        {message && <Notice>{message}</Notice>}
        {dirty && (
          <p className="mt-2 text-sm text-muted-foreground">
            Save your changes before archiving.
          </p>
        )}
      </div>
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
