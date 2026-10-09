import { expect, test } from 'vitest';
import { issueRouteParams, issueTitleSlug } from './model';

test('issue title URLs handle accents, non-Latin text, separators, and symbol-only titles', () => {
  expect(issueTitleSlug('  Ship the issue workflow!  ')).toBe(
    'ship-the-issue-workflow',
  );
  expect(issueTitleSlug('Café / 日本語 & C++')).toBe('cafe-日本語-c');
  expect(issueTitleSlug('!!!')).toBe('issue');
  expect(
    issueRouteParams('product', { identifier: 'ENG-42', title: 'Fix links' }),
  ).toEqual({
    workspaceSlug: 'product',
    issueKey: 'ENG-42',
    issueTitle: 'fix-links',
  });
});
