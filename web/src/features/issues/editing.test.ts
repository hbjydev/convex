import { afterEach, expect, test, vi } from 'vitest';
import { createIssueEditing } from './editing';
import type { Issue } from './model';
const original: Issue = {
  id: 'issue',
  workspace_id: 'space',
  team_id: 'team',
  status_id: 'todo',
  creator_membership_id: 'owner',
  title: 'Original',
  description: 'Saved source',
  identifier: 'ENG-1',
  number: 1,
  priority: 'none',
  assignee_membership_id: null,
  version: 1,
  created_at: '',
  updated_at: '',
};
afterEach(() => vi.useRealTimers());

test('autosaves after a pause and sends no update on arrival', async () => {
  vi.useFakeTimers();
  const persist = vi.fn(async () => ({
    issue: { ...original, title: 'Changed', version: 2 },
  }));
  const editor = createIssueEditing(original, persist, vi.fn());
  await vi.advanceTimersByTimeAsync(1000);
  expect(persist).not.toHaveBeenCalled();
  editor.change({ title: 'Changed' });
  await vi.advanceTimersByTimeAsync(799);
  expect(persist).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(1);
  expect(persist).toHaveBeenCalledWith({
    expected_version: 1,
    title: 'Changed',
  });
  expect(editor.dirty()).toBe(false);
  editor.dispose();
});

test('serializes writes and preserves edits made during an in-flight save', async () => {
  let finish: ((value: { issue: Issue }) => void) | undefined;
  const first = new Promise<{ issue: Issue }>((resolve) => {
    finish = resolve;
  });
  const persist = vi
    .fn()
    .mockReturnValueOnce(first)
    .mockResolvedValueOnce({
      issue: { ...original, title: 'First', description: 'Latest', version: 3 },
    });
  const editor = createIssueEditing(original, persist, vi.fn());
  editor.change({ title: 'First' }, true);
  editor.change({ description: 'Latest' }, true);
  expect(persist).toHaveBeenCalledTimes(1);
  finish?.({ issue: { ...original, title: 'First', version: 2 } });
  await editor.save();
  expect(persist).toHaveBeenNthCalledWith(2, {
    expected_version: 2,
    description: 'Latest',
  });
  expect(editor.getSnapshot().draft.description).toBe('Latest');
  expect(editor.dirty()).toBe(false);
  editor.dispose();
});

test('failure retains the latest draft and pauses until retry', async () => {
  vi.useFakeTimers();
  const persist = vi
    .fn()
    .mockRejectedValueOnce(new Error('Offline'))
    .mockResolvedValueOnce({
      issue: { ...original, title: 'Draft', version: 2 },
    });
  const editor = createIssueEditing(original, persist, vi.fn());
  editor.change({ title: 'Draft' });
  expect(await editor.save()).toBe(false);
  await vi.advanceTimersByTimeAsync(5000);
  expect(persist).toHaveBeenCalledTimes(1);
  expect(editor.getSnapshot().draft.title).toBe('Draft');
  expect(await editor.save(true)).toBe(true);
  expect(editor.getSnapshot().error).toBe('');
  editor.dispose();
});

test('conflicts require review before retrying with the new version', async () => {
  vi.useFakeTimers();
  const current = { ...original, title: 'Other title', version: 2 };
  const persist = vi
    .fn()
    .mockResolvedValueOnce({ conflict: current })
    .mockResolvedValueOnce({
      issue: { ...current, priority: 'high', version: 3 },
    });
  const editor = createIssueEditing(original, persist, vi.fn());
  editor.change({ priority: 'high' });
  expect(await editor.save()).toBe(false);
  editor.resolve(true);
  expect(editor.getSnapshot().draft.title).toBe('Other title');
  expect(editor.getSnapshot().draft.priority).toBe('high');
  await vi.advanceTimersByTimeAsync(5000);
  expect(persist).toHaveBeenCalledTimes(1);
  await editor.save(true);
  expect(persist).toHaveBeenLastCalledWith({
    expected_version: 2,
    priority: 'high',
  });
  editor.dispose();
});

test('invalid drafts are kept without sending a request', async () => {
  const persist = vi.fn();
  const editor = createIssueEditing(original, persist, vi.fn());
  editor.change({ description: 'x'.repeat(50001) });
  expect(await editor.save()).toBe(false);
  expect(editor.getSnapshot().error).toContain('50,000');
  expect(editor.dirty()).toBe(true);
  expect(persist).not.toHaveBeenCalled();
  editor.dispose();
});

test('archive flushes edits and prevents new edits during the archive write', async () => {
  let finish: ((value: { issue: Issue }) => void) | undefined;
  const persist = vi
    .fn()
    .mockResolvedValueOnce({
      issue: { ...original, title: 'Saved before archive', version: 2 },
    })
    .mockImplementationOnce(
      () =>
        new Promise<{ issue: Issue }>((resolve) => {
          finish = resolve;
        }),
    );
  const editor = createIssueEditing(original, persist, vi.fn());
  editor.change({ title: 'Saved before archive' });
  const archived = editor.archive();
  await vi.waitFor(() => expect(editor.getSnapshot().archiving).toBe(true));
  editor.change({ description: 'Too late' });
  expect(editor.getSnapshot().draft.description).toBe(original.description);
  expect(persist).toHaveBeenNthCalledWith(2, {
    expected_version: 2,
    archived: true,
  });
  finish?.({
    issue: {
      ...original,
      title: 'Saved before archive',
      version: 3,
      archived_at: 'now',
    },
  });
  expect(await archived).toBe(true);
  expect(editor.dirty()).toBe(false);
  expect(editor.getSnapshot().archiving).toBe(false);
  editor.dispose();
});
