import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { IssueDescription } from './description';

describe('issue descriptions', () => {
  it('renders Markdown structure and GFM without changing the source', () => {
    const source =
      '## Acceptance\n\nKeep **formatting** and `code`.\n\n- [x] Done\n- [ ] Pending\n\n| Name | State |\n| --- | --- |\n| API | Ready |';
    const html = renderToStaticMarkup(
      <IssueDescription description={source} />,
    );
    expect(html).toContain('<h2>Acceptance</h2>');
    expect(html).toContain('<strong>formatting</strong>');
    expect(html).toContain('<code>code</code>');
    expect(html).toContain('type="checkbox" disabled="" checked=""');
    expect(html).toContain('<table>');
    expect(html).toContain('<td>Ready</td>');
  });

  it('ignores raw HTML and blocks executable link protocols', () => {
    const html = renderToStaticMarkup(
      <IssueDescription
        description={
          '<script>alert(1)</script>\n\n<img src="x" onerror="alert(1)">\n\n[Unsafe](javascript:alert%281%29)\n\n[Docs](https://example.com)'
        }
      />,
    );
    expect(html).not.toContain('<script');
    expect(html).not.toContain('onerror');
    expect(html).not.toContain('javascript:');
    expect(html).toContain('href="https://example.com"');
  });
});
