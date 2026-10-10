import { MarkdownManager } from '@tiptap/markdown';
import { expect, test } from 'vitest';
import { descriptionExtensions, parseDescription } from './description-schema';
test('round-trips existing Markdown structure through ProseMirror', () => {
  const source =
    '## Heading\n\n**Bold** and `code`.\n\n- [x] Done\n- [ ] Pending\n\n| Property | Value |\n| --- | --- |\n| Status | Ready |\n\n```json\n{"ok":true}\n```\n\n[Docs](https://example.com)';
  const document = parseDescription(source);
  const markdown = new MarkdownManager({
    extensions: descriptionExtensions,
  }).serialize(document);
  expect(markdown).toContain('## Heading');
  expect(markdown).toContain('**Bold**');
  expect(markdown).toContain('[x] Done');
  expect(markdown).toContain('[ ] Pending');
  expect(markdown).toMatch(/\| Status\s+\| Ready\s+\|/);
  expect(markdown).toContain('```json');
  expect(markdown).toContain('[Docs](https://example.com)');
  expect(parseDescription(markdown)).toEqual(document);
});
test('filters unsafe Markdown image and link URLs', () => {
  const document = JSON.stringify(
    parseDescription(
      '[Unsafe](javascript:alert%281%29)\n\n![Unsafe](javascript:alert%281%29)',
    ),
  );
  expect(document).not.toContain('javascript:');
});
