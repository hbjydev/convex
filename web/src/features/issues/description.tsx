import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
// oxlint-disable jsx-a11y/no-noninteractive-tabindex -- Scrollable tables need keyboard access.

export function IssueDescription({ description }: { description: string }) {
  return (
    <div className="typeset issue-description">
      <Markdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
          table: ({ children }) => (
            <div
              className="typeset-scroll"
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
    </div>
  );
}
