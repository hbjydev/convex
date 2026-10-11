import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prose } from '@/components/prose';
// oxlint-disable jsx-a11y/no-noninteractive-tabindex -- Scrollable tables need keyboard access.

export function IssueDescription({ description }: { description: string }) {
  return (
    <Prose>
      <Markdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
          table: ({ children }) => (
            <div
              className="mt-[1.25em] overflow-x-auto [&>table]:mt-0 [&>table]:w-max [&>table]:max-w-none"
              tabIndex={0}
              role="region"
              aria-label="Table"
            >
              <table>{children}</table>
            </div>
          ),
          img: ({ src, alt, title }) => (
            <img
              src={src}
              alt={alt ?? ''}
              title={title}
              loading="lazy"
              referrerPolicy="no-referrer"
            />
          ),
        }}
      >
        {description}
      </Markdown>
    </Prose>
  );
}
