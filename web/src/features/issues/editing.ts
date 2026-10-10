import { issuePatch, type Draft, type Issue, type IssuePatch } from './model';

export type SaveResult = { issue: Issue } | { conflict: Issue };
export type EditingState = {
  issue: Issue;
  draft: Draft;
  pending: boolean;
  archiving: boolean;
  error: string;
  conflict?: Issue;
  review: boolean;
};

export function mergePendingDraft(
  saved: Issue,
  sent: Draft,
  current: Draft,
): Draft {
  return {
    title: current.title === sent.title ? saved.title : current.title,
    description:
      current.description === sent.description
        ? saved.description
        : current.description,
    status_id:
      current.status_id === sent.status_id
        ? saved.status_id
        : current.status_id,
    priority:
      current.priority === sent.priority ? saved.priority : current.priority,
    assignee_membership_id:
      current.assignee_membership_id === sent.assignee_membership_id
        ? saved.assignee_membership_id
        : current.assignee_membership_id,
  };
}

export function createIssueEditing(
  initial: Issue,
  persist: (patch: IssuePatch) => Promise<SaveResult>,
  onSaved: (issue: Issue) => void,
) {
  let state: EditingState = {
    issue: initial,
    draft: initial,
    pending: false,
    archiving: false,
    error: '',
    review: false,
  };
  const listeners = new Set<() => void>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let flight: Promise<boolean> | undefined;
  let disposed = false;
  const dirty = () =>
    Object.keys(issuePatch(state.issue, state.draft)).length > 1;
  const notify = (next: Partial<EditingState>) => {
    state = { ...state, ...next };
    listeners.forEach((listener) => listener());
  };
  const clearTimer = () => {
    clearTimeout(timer);
    timer = undefined;
  };
  async function run(archive?: boolean) {
    notify({ pending: true, archiving: archive !== undefined, error: '' });
    try {
      do {
        const sent = { ...state.draft };
        const patch =
          archive === undefined
            ? issuePatch(state.issue, sent)
            : { expected_version: state.issue.version, archived: archive };
        if (archive === undefined && !dirty()) return true;
        if (
          archive === undefined &&
          (!sent.title.trim() ||
            sent.title.trim().length > 240 ||
            sent.description.length > 50000)
        ) {
          notify({
            error: !sent.title.trim()
              ? 'Add an issue title before saving.'
              : sent.description.length > 50000
                ? 'Descriptions must be 50,000 characters or fewer.'
                : 'Titles must be 240 characters or fewer.',
          });
          return false;
        }
        const result = await persist(patch);
        if ('conflict' in result) {
          notify({ conflict: result.conflict });
          return false;
        }
        notify({
          issue: result.issue,
          draft: mergePendingDraft(result.issue, sent, state.draft),
          review: false,
        });
        if (!disposed) onSaved(result.issue);
        if (archive !== undefined) return true;
      } while (dirty());
      return true;
    } catch (reason) {
      notify({
        error:
          reason instanceof Error
            ? reason.message
            : 'Could not save this issue. Try again.',
      });
      return false;
    } finally {
      notify({ pending: false, archiving: false });
    }
  }
  function save(force = false, archive?: boolean): Promise<boolean> {
    clearTimer();
    if (flight) return flight;
    if (
      state.conflict ||
      (!force && (state.error || state.review)) ||
      (state.issue.archived_at && archive !== false)
    )
      return Promise.resolve(!dirty());
    flight = run(archive).finally(() => {
      flight = undefined;
    });
    return flight;
  }
  return {
    getSnapshot: () => state,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    dirty,
    save,
    change: (patch: Partial<Draft>, immediate = false) => {
      if (state.issue.archived_at || state.conflict || state.archiving) return;
      notify({ draft: { ...state.draft, ...patch }, error: '' });
      clearTimer();
      if (state.review) return;
      if (immediate) void save();
      else
        timer = setTimeout(() => {
          void save();
        }, 800);
    },
    resolve: (keepChanges: boolean) => {
      const current = state.conflict;
      if (!current) return;
      clearTimer();
      const draft = keepChanges
        ? mergePendingDraft(current, state.issue, state.draft)
        : current;
      notify({
        issue: current,
        draft,
        conflict: undefined,
        error: '',
        review: keepChanges,
      });
      onSaved(current);
    },
    discard: () => {
      clearTimer();
      notify({ draft: state.issue, error: '', review: false });
    },
    archive: async () => {
      if (
        !(await save()) ||
        dirty() ||
        state.conflict ||
        state.review ||
        state.error
      )
        return false;
      return save(true, !state.issue.archived_at);
    },
    dispose: () => {
      disposed = true;
      clearTimer();
    },
    activate: () => {
      disposed = false;
    },
  };
}
