import { EditorContent, useEditor } from '@tiptap/react';
import { useEffect, useRef, useState } from 'react';
import { descriptionExtensions, parseDescription } from './description-schema';
import { SelectionToolbar } from './selection-toolbar';

export function RichDescription({
  value,
  onChange,
  onBlur,
  disabled,
}: {
  value: string;
  onChange: (value: string) => void;
  onBlur: () => void;
  disabled: boolean;
}) {
  const lastValue = useRef(value);
  const [initialContent] = useState(() => parseDescription(value));
  const editor = useEditor({
    extensions: descriptionExtensions,
    content: initialContent,
    editable: !disabled,
    immediatelyRender: true,
    editorProps: {
      handleKeyDown: (_view, event) => {
        if (event.altKey && event.key === 'F10') {
          const button = document.querySelector<HTMLButtonElement>(
            '[data-description-toolbar] button',
          );
          if (button) {
            button.focus();
            return true;
          }
        }
        return false;
      },
      attributes: {
        class: 'typeset issue-description issue-prosemirror',
        role: 'textbox',
        'aria-label': 'Description',
        'aria-multiline': 'true',
        'aria-description':
          'Select text to format it. Press Alt+F10 to focus the formatting toolbar.',
        'aria-readonly': disabled ? 'true' : 'false',
        spellcheck: 'true',
      },
    },
    onUpdate: ({ editor }) => {
      const markdown = editor.getMarkdown();
      lastValue.current = markdown;
      onChange(markdown);
    },
    onBlur,
  });
  useEffect(() => {
    editor?.setEditable(!disabled, false);
  }, [editor, disabled]);
  useEffect(() => {
    if (editor && value !== lastValue.current) {
      editor.commands.setContent(parseDescription(value), {
        emitUpdate: false,
      });
      lastValue.current = value;
    }
  }, [editor, value]);
  return (
    <>
      <EditorContent editor={editor} />
      {editor && !disabled && <SelectionToolbar editor={editor} />}
    </>
  );
}
