import { useEditorState, type Editor } from '@tiptap/react';
import { BubbleMenu } from '@tiptap/react/menus';
import { TextSelection } from '@tiptap/pm/state';
import { useCallback, useState, type ReactNode } from 'react';
import {
  BoldIcon,
  ItalicIcon,
  StrikethroughIcon,
  CodeIcon,
  LinkIcon,
  QuoteIcon,
  ListIcon,
  ChevronDownIcon,
  CaseSensitiveIcon,
  RemoveFormattingIcon,
} from 'lucide-react';
import { defaultUrlTransform } from 'react-markdown';
import { Button } from '@/components/ui/button';
import {
  ButtonGroup,
  ButtonGroupSeparator,
} from '@/components/ui/button-group';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
} from '@/components/ui/dropdown-menu';
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
} from '@/components/ui/popover';
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldError,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from '@/components/ui/tooltip';

const placement = {
  strategy: 'fixed' as const,
  placement: 'top' as const,
  offset: 8,
  flip: { padding: 12 },
  shift: { padding: 12 },
};

const appendToBody = () => document.body;

function FormatButton({
  label,
  active,
  disabled,
  onClick,
  children,
}: {
  label: string;
  active: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip delayDuration={400}>
      <TooltipTrigger asChild>
        <Button
          type="button"
          size="icon"
          variant={active ? 'secondary' : 'ghost'}
          aria-label={label}
          aria-pressed={active}
          disabled={disabled}
          onMouseDown={(event) => event.preventDefault()}
          onClick={onClick}
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

export function SelectionToolbar({ editor }: { editor: Editor }) {
  const [menu, setMenu] = useState<'style' | 'list' | undefined>();
  const [linkOpen, setLinkOpen] = useState(false);
  const [url, setUrl] = useState('');
  const [error, setError] = useState('');
  const selected = useEditorState({
    editor,
    selector: ({ editor }) => ({
      bold: editor.isActive('bold'),
      italic: editor.isActive('italic'),
      strike: editor.isActive('strike'),
      code: editor.isActive('code'),
      quote: editor.isActive('blockquote'),
      link: editor.isActive('link'),
      style:
        [1, 2, 3]
          .find((level) => editor.isActive('heading', { level }))
          ?.toString() ?? 'paragraph',
      list: editor.isActive('taskList')
        ? 'task'
        : editor.isActive('orderedList')
          ? 'ordered'
          : editor.isActive('bulletList')
            ? 'bullet'
            : 'none',
      canBold: editor.can().toggleBold(),
      canItalic: editor.can().toggleItalic(),
      canStrike: editor.can().toggleStrike(),
      canCode: editor.can().toggleCode(),
    }),
  });
  const selectedLine = useCallback(() => {
    const { from, to } = editor.state.selection;
    const start = editor.view.domAtPos(from);
    const end = editor.view.domAtPos(to);
    const range = document.createRange();
    range.setStart(start.node, start.offset);
    range.setEnd(end.node, end.offset);
    const rects = [...range.getClientRects()].filter(
      (rect) => rect.width && rect.height,
    );
    const first = rects[0];
    if (!first) return null;
    const line = rects.filter((rect) => Math.abs(rect.top - first.top) < 3);
    const left = Math.min(...line.map((rect) => rect.left));
    const right = Math.max(...line.map((rect) => rect.right));
    const rect = new DOMRect(left, first.top, right - left, first.height);
    return {
      getBoundingClientRect: () => rect,
      contextElement: editor.view.dom,
    };
  }, [editor]);
  const shouldShow = useCallback(
    ({
      editor,
      element,
      state,
    }: {
      editor: Editor;
      element: HTMLElement;
      state: Editor['state'];
    }) =>
      editor.isEditable &&
      state.selection instanceof TextSelection &&
      !state.selection.empty &&
      !!state.doc
        .textBetween(state.selection.from, state.selection.to)
        .trim() &&
      (editor.isFocused ||
        !!menu ||
        linkOpen ||
        element.contains(document.activeElement)),
    [menu, linkOpen],
  );
  const returnToEditor = (event: Event) => {
    event.preventDefault();
    editor.commands.focus(undefined, { scrollIntoView: false });
  };
  const setStyle = (value: string) => {
    const chain = editor.chain().focus(undefined, { scrollIntoView: false });
    if (value === 'paragraph') chain.setParagraph().run();
    else if (value === '1' || value === '2' || value === '3') {
      chain
        .setHeading({ level: value === '1' ? 1 : value === '2' ? 2 : 3 })
        .run();
    }
  };
  return (
    <BubbleMenu
      editor={editor}
      updateDelay={100}
      options={placement}
      appendTo={appendToBody}
      getReferencedVirtualElement={selectedLine}
      className="w-max max-w-[calc(100vw-24px)] z-50"
      shouldShow={shouldShow}
    >
      <ButtonGroup
        role="toolbar"
        aria-label="Description formatting"
        data-description-toolbar
        className="max-w-[calc(100vw-24px)] flex-wrap items-center rounded-lg border bg-popover p-1 text-popover-foreground shadow-[0_4px_12px_color-mix(in_oklch,var(--foreground)_12%,transparent)] [&_[data-slot=button-group-separator]]:min-h-6 max-md:[&_button]:min-h-11 max-md:[&_button]:min-w-11"
        onKeyDown={(event) => {
          if (
            !(event.target instanceof HTMLElement) ||
            !event.currentTarget.contains(event.target)
          )
            return;
          if (event.key === 'Escape') {
            event.preventDefault();
            event.stopPropagation();
            editor
              .chain()
              .setTextSelection(editor.state.selection.to)
              .focus()
              .run();
          } else if (
            ['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)
          ) {
            const buttons = [
              ...event.currentTarget.querySelectorAll<HTMLButtonElement>(
                'button:not(:disabled)',
              ),
            ];
            if (!(event.target instanceof HTMLButtonElement)) return;
            const current = buttons.indexOf(event.target);
            const index =
              event.key === 'Home'
                ? 0
                : event.key === 'End'
                  ? buttons.length - 1
                  : (current +
                      (event.key === 'ArrowRight' ? 1 : -1) +
                      buttons.length) %
                    buttons.length;
            event.preventDefault();
            event.stopPropagation();
            buttons[index]?.focus();
          }
        }}
      >
        <ButtonGroup aria-label="Text style">
          <DropdownMenu
            open={menu === 'style'}
            onOpenChange={(open) => setMenu(open ? 'style' : undefined)}
          >
            <DropdownMenuTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                aria-label="Text style"
              >
                <CaseSensitiveIcon data-icon="inline-start" />
                <ChevronDownIcon data-icon="inline-end" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent onCloseAutoFocus={returnToEditor}>
              <DropdownMenuRadioGroup
                value={selected.style}
                onValueChange={setStyle}
              >
                <DropdownMenuRadioItem value="paragraph">
                  Normal text
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="1">
                  Heading 1
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="2">
                  Heading 2
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="3">
                  Heading 3
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>
        </ButtonGroup>
        <ButtonGroupSeparator />
        <ButtonGroup aria-label="Inline formatting">
          <FormatButton
            label="Bold"
            active={selected.bold}
            disabled={!selected.canBold}
            onClick={() => editor.chain().focus().toggleBold().run()}
          >
            <BoldIcon />
          </FormatButton>
          <FormatButton
            label="Italic"
            active={selected.italic}
            disabled={!selected.canItalic}
            onClick={() => editor.chain().focus().toggleItalic().run()}
          >
            <ItalicIcon />
          </FormatButton>
          <FormatButton
            label="Strikethrough"
            active={selected.strike}
            disabled={!selected.canStrike}
            onClick={() => editor.chain().focus().toggleStrike().run()}
          >
            <StrikethroughIcon />
          </FormatButton>
          <FormatButton
            label="Inline code"
            active={selected.code}
            disabled={!selected.canCode}
            onClick={() => editor.chain().focus().toggleCode().run()}
          >
            <CodeIcon />
          </FormatButton>
          <Popover
            open={linkOpen}
            onOpenChange={(open) => {
              setLinkOpen(open);
              if (open) {
                setUrl(editor.getAttributes('link').href ?? '');
                setError('');
              }
            }}
          >
            <PopoverTrigger asChild>
              <Button
                type="button"
                size="icon"
                variant={selected.link ? 'secondary' : 'ghost'}
                aria-label="Link"
                onMouseDown={(event) => event.preventDefault()}
              >
                <LinkIcon />
              </Button>
            </PopoverTrigger>
            <PopoverContent
              aria-label="Edit link"
              onCloseAutoFocus={returnToEditor}
            >
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  const href = url.trim();
                  if (
                    href &&
                    (defaultUrlTransform(href) !== href ||
                      !editor.can().setLink({ href }))
                  ) {
                    setError(
                      'Enter a valid link, such as https://example.com.',
                    );
                    return;
                  }
                  const chain = editor.chain().focus().extendMarkRange('link');
                  if (href) chain.setLink({ href }).run();
                  else chain.unsetLink().run();
                  setLinkOpen(false);
                }}
              >
                <FieldGroup>
                  <Field data-invalid={!!error}>
                    <FieldLabel htmlFor="issue-description-link">
                      Link URL
                    </FieldLabel>
                    <Input
                      id="issue-description-link"
                      value={url}
                      onChange={(event) => {
                        setUrl(event.target.value);
                        setError('');
                      }}
                      placeholder="https://example.com"
                      aria-invalid={!!error}
                      aria-describedby={error ? 'issue-link-error' : undefined}
                    />
                    {error && (
                      <FieldError id="issue-link-error">{error}</FieldError>
                    )}
                  </Field>
                  <ButtonGroup aria-label="Link actions">
                    <Button type="submit" size="sm">
                      Apply link
                    </Button>
                    {selected.link && (
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        onClick={() => {
                          editor
                            .chain()
                            .focus()
                            .extendMarkRange('link')
                            .unsetLink()
                            .run();
                          setLinkOpen(false);
                        }}
                      >
                        Remove link
                      </Button>
                    )}
                  </ButtonGroup>
                </FieldGroup>
              </form>
            </PopoverContent>
          </Popover>
        </ButtonGroup>
        <ButtonGroupSeparator />
        <ButtonGroup aria-label="Block formatting">
          <FormatButton
            label="Quote"
            active={selected.quote}
            onClick={() => editor.chain().focus().toggleBlockquote().run()}
          >
            <QuoteIcon />
          </FormatButton>
          <DropdownMenu
            open={menu === 'list'}
            onOpenChange={(open) => setMenu(open ? 'list' : undefined)}
          >
            <DropdownMenuTrigger asChild>
              <Button
                type="button"
                variant={selected.list !== 'none' ? 'secondary' : 'ghost'}
                size="sm"
                aria-label="List style"
              >
                <ListIcon data-icon="inline-start" />
                <ChevronDownIcon data-icon="inline-end" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent onCloseAutoFocus={returnToEditor}>
              <DropdownMenuRadioGroup
                value={selected.list}
                onValueChange={(value) => {
                  const chain = editor.chain().focus();
                  if (value === 'bullet') chain.toggleBulletList().run();
                  else if (value === 'ordered') chain.toggleOrderedList().run();
                  else if (value === 'task') chain.toggleTaskList().run();
                  else chain.clearNodes().run();
                }}
              >
                <DropdownMenuRadioItem value="none">
                  No list
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="bullet">
                  Bulleted list
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="ordered">
                  Numbered list
                </DropdownMenuRadioItem>
                <DropdownMenuRadioItem value="task">
                  Checklist
                </DropdownMenuRadioItem>
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>
          <FormatButton
            label="Clear formatting"
            active={false}
            onClick={() =>
              editor.chain().focus().unsetAllMarks().clearNodes().run()
            }
          >
            <RemoveFormattingIcon />
          </FormatButton>
        </ButtonGroup>
      </ButtonGroup>
    </BubbleMenu>
  );
}
