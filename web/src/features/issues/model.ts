import type { components } from '@/api/schema';
export type Issue = components['schemas']['Issue'];
export type IssuePatch = components['schemas']['IssuePatch'];
export type WorkspaceDetail = components['schemas']['WorkspaceDetail'];
export type Draft = Pick<
  Issue,
  'title' | 'description' | 'status_id' | 'priority' | 'assignee_membership_id'
>;
export const priorities: Issue['priority'][] = [
  'none',
  'urgent',
  'high',
  'medium',
  'low',
];
export function issueTitleSlug(title: string) {
  return (
    title
      .normalize('NFKD')
      .replace(/\p{M}/gu, '')
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, '-')
      .replace(/^-|-$/g, '') || 'issue'
  );
}
export function issueRouteParams(
  slug: string,
  issue: Pick<Issue, 'identifier' | 'title'>,
) {
  return {
    workspaceSlug: slug,
    issueKey: issue.identifier,
    issueTitle: issueTitleSlug(issue.title),
  };
}
export function issuePatch(original: Issue, draft: Draft): IssuePatch {
  const patch: IssuePatch = { expected_version: original.version };
  if (draft.title.trim() !== original.title) patch.title = draft.title.trim();
  if (draft.description !== original.description)
    patch.description = draft.description;
  if (draft.status_id !== original.status_id) patch.status_id = draft.status_id;
  if (draft.priority !== original.priority) patch.priority = draft.priority;
  if (draft.assignee_membership_id !== original.assignee_membership_id)
    patch.assignee_membership_id = draft.assignee_membership_id;
  return patch;
}
