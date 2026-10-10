import StarterKit from '@tiptap/starter-kit';
import { Markdown, MarkdownManager } from '@tiptap/markdown';
import { TaskItem, TaskList } from '@tiptap/extension-list';
import { TableKit } from '@tiptap/extension-table';
import Image from '@tiptap/extension-image';
import { Placeholder } from '@tiptap/extensions';
import type { JSONContent } from '@tiptap/react';
import { defaultUrlTransform } from 'react-markdown';
export const descriptionExtensions = [
  StarterKit.configure({
    underline: false,
    link: { openOnClick: false },
    trailingNode: false,
  }),
  TaskList,
  TaskItem.configure({
    nested: true,
    HTMLAttributes: { 'data-type': 'taskItem' },
  }),
  TableKit.configure({ table: { resizable: false } }),
  Image.configure({
    HTMLAttributes: { loading: 'lazy', referrerpolicy: 'no-referrer' },
  }),
  Placeholder.configure({ placeholder: 'Add a description…' }),
  Markdown.configure({ markedOptions: { gfm: true } }),
];

function safeUrls(node: JSONContent): JSONContent {
  const attrs = node.attrs ? { ...node.attrs } : undefined;
  if (attrs && typeof attrs.src === 'string')
    attrs.src = defaultUrlTransform(attrs.src);
  return {
    ...node,
    attrs,
    marks: node.marks?.map((mark) => ({
      ...mark,
      attrs:
        mark.attrs && typeof mark.attrs.href === 'string'
          ? { ...mark.attrs, href: defaultUrlTransform(mark.attrs.href) }
          : mark.attrs,
    })),
    content: node.content?.map(safeUrls),
  };
}
export function parseDescription(source: string): JSONContent {
  return safeUrls(
    new MarkdownManager({
      extensions: descriptionExtensions,
      markedOptions: { gfm: true },
    }).parse(source),
  );
}
