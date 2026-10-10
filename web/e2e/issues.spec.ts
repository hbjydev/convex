import { expect, test } from '@playwright/test';
import type { components } from '../src/api/schema';
const themes: ('light' | 'dark')[] = ['light', 'dark'];
const password = 'a secure browser test password';
const headers = { Origin: 'http://127.0.0.1:5174', 'X-Convex-CSRF': '1' };

test('two members create, edit, reconcile, archive, and restore an issue', async ({
  page,
  browser,
  request,
}) => {
  test.setTimeout(90_000);
  if (
    (await (await request.get('/api/v1/auth/bootstrap')).json()).setup_required
  ) {
    const setup = await request.post('/api/v1/auth/setup', {
      headers,
      data: {
        setup_token: 'browser-test-installer-credential-12345',
        email: 'browser-owner@example.com',
        password,
        display_name: 'Installation owner',
        workspace_name: 'Initial workspace',
        workspace_slug: 'initial-workspace',
        team_name: 'Engineering',
        team_key: 'ENG',
      },
    });
    expect(setup.ok()).toBe(true);
  }
  await page.goto('/auth/login');
  await page
    .getByLabel('Email', { exact: true })
    .fill('browser-owner@example.com');
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Your workspaces', exact: true }),
  ).toBeVisible();
  const slug = `issues-${test.info().project.name}-${Date.now()}`;
  const response = await page.request.post('/api/v1/workspaces', {
    headers,
    data: {
      name: 'Issue workspace',
      slug,
      team_name: 'Engineering',
      team_key: 'ENG',
    },
  });
  expect(response.ok()).toBe(true);
  const workspace = await response.json();
  const invitation = await page.request.post(
    `/api/v1/workspaces/${workspace.id}/invitations`,
    { headers, data: { role: 'member' } },
  );
  const token = (await invitation.json()).url.split('#')[1];
  const context = await browser.newContext({
    viewport: page.viewportSize() ?? undefined,
  });
  const second = await context.newPage();
  const joined = await second.request.post(
    'http://127.0.0.1:5174/api/v1/invitations/accept',
    {
      headers,
      data: {
        token,
        email: `${slug}@example.com`,
        password,
        display_name: 'Second member',
      },
    },
  );
  expect(joined.ok()).toBe(true);
  await page.goto(`/${slug}/team/eng/all`);
  await expect(page.getByText('Capture your first issue')).toBeVisible();
  await page.getByRole('button', { name: 'New issue', exact: true }).click();
  await page
    .getByLabel('Title', { exact: true })
    .fill('Ship the issue workflow');
  await page
    .getByLabel('Description', { exact: true })
    .fill('## Acceptance\nKeep **Markdown** through reload.');
  await page
    .getByRole('button', { name: 'Assignee: Assignee', exact: true })
    .click();
  await page
    .getByRole('menuitemradio', { name: 'Second member', exact: true })
    .click();
  await expect(page.getByRole('menuitemradio')).toHaveCount(0);
  await page.screenshot({
    path: `test-results/issue-create-${test.info().project.name}.png`,
    fullPage: true,
  });
  await page.getByRole('button', { name: 'Create issue', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Ship the issue workflow', exact: true }),
  ).toBeVisible();
  const issueUrl = page.url();
  await expect(page).toHaveURL(
    new RegExp(`/${slug}/issue/ENG-1/ship-the-issue-workflow$`),
  );
  const initialIssue: components['schemas']['Issue'] = await (
    await page.request.get(`/api/v1/workspaces/${workspace.id}/issues/ENG-1`)
  ).json();
  const issueId = initialIssue.id;
  const content = page.getByRole('article', { name: 'Issue content' });
  const description = page.getByRole('textbox', {
    name: 'Description',
    exact: true,
  });
  const saved = () => expect(page.getByRole('status')).toHaveText('Saved');
  const choose = async (
    target: typeof page,
    property: string,
    value: string,
  ) => {
    await target
      .getByRole('button', { name: new RegExp(`^${property}:`) })
      .click();
    await target
      .getByRole('menuitemradio', { name: value, exact: true })
      .click();
  };
  await expect(
    content.getByRole('heading', { name: 'Acceptance' }),
  ).toBeVisible();
  await expect(content.locator('strong')).toHaveText('Markdown');
  await expect(description).toHaveAttribute('contenteditable', 'true');
  await expect(
    page.getByRole('button', { name: 'Edit issue', exact: true }),
  ).toHaveCount(0);
  // Opening an issue must not rewrite or version its Markdown.
  await saved();
  expect(
    (
      await (
        await page.request.get(
          `/api/v1/workspaces/${workspace.id}/issues/ENG-1`,
        )
      ).json()
    ).version,
  ).toBe(1);
  await second.goto(issueUrl);
  await page
    .getByRole('textbox', { name: 'Title', exact: true })
    .fill('Deliver the issue workflow');
  await saved();
  await expect(page).toHaveURL(
    new RegExp(`/${slug}/issue/ENG-1/deliver-the-issue-workflow$`),
  );
  // Properties are editable in the reading layout and still detect stale versions.
  await choose(second, 'Priority', 'High');
  await expect(second.getByRole('alert')).toContainText(
    'Your draft is preserved',
  );
  await expect(
    second.getByRole('button', { name: 'Priority: High', exact: true }),
  ).toBeDisabled();
  await expect(
    second.getByRole('button', { name: 'Use current version', exact: true }),
  ).toBeFocused();
  await second.screenshot({
    path: `test-results/issue-conflict-${test.info().project.name}.png`,
    fullPage: true,
  });
  await second
    .getByRole('button', { name: 'Keep my changed fields for review' })
    .click();
  await expect(
    second.getByRole('textbox', { name: 'Title', exact: true }),
  ).toHaveValue('Deliver the issue workflow');
  await expect(second.getByRole('status')).toHaveText('Review changes');
  await second
    .getByRole('button', { name: 'Save reviewed changes', exact: true })
    .click();
  await expect(second.getByRole('status')).toHaveText('Saved');
  await page
    .getByRole('textbox', { name: 'Title', exact: true })
    .fill('My alternate title');
  await expect(page.getByRole('alert')).toContainText(
    'Your draft is preserved',
  );
  await expect(
    page.getByRole('textbox', { name: 'Title', exact: true }),
  ).toHaveValue('My alternate title');
  await page.getByRole('button', { name: 'Use current version' }).click();
  await expect(
    page.getByRole('textbox', { name: 'Title', exact: true }),
  ).toHaveValue('Deliver the issue workflow');
  // Failed autosaves preserve the content and guard navigation away.
  await page.route('**/api/v1/workspaces/*/issues/*', (route) =>
    route.request().method() === 'PATCH' ? route.abort() : route.continue(),
  );
  await description.fill('A draft retained after a failed autosave.');
  await expect(page.getByRole('alert')).toContainText('Check your connection');
  await expect(description).toHaveText(
    'A draft retained after a failed autosave.',
  );
  await page.getByRole('link', { name: 'Back to issues' }).click();
  await expect(
    page.getByRole('dialog', { name: 'Unsaved changes' }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Stay on issue' }).click();
  await expect(description).toHaveText(
    'A draft retained after a failed autosave.',
  );
  await page.unroute('**/api/v1/workspaces/*/issues/*');
  await page.getByRole('button', { name: 'Retry save' }).click();
  await saved();
  await page.reload();
  await expect(description).toHaveText(
    'A draft retained after a failed autosave.',
  );
  await choose(page, 'Assignee', 'Unassigned');
  await saved();
  await choose(page, 'Status', 'Done');
  await saved();
  // Seed existing rich Markdown and edit it without switching modes or losing structure.
  const current = await (
    await page.request.get(`/api/v1/workspaces/${workspace.id}/issues/ENG-1`)
  ).json();
  const formattedDescription =
    '## What to build\n\nRender **Markdown** with a readable layout and keep the original source for editing.\n\n### Acceptance criteria\n\n- [x] Preserve saved descriptions after reload\n- [ ] Keep status and assignee easy to find\n\nSee the [documentation](https://example.com/docs) and use `expected_version` when saving.\n\n> Failed requests should preserve your draft.\n\n```json\n{ "expected_version": 2 }\n```\n\n| Property | Behavior |\n| --- | --- |\n| Status | Scoped to the team |\n| Assignee | Workspace member |';
  expect(
    (
      await page.request.patch(
        `/api/v1/workspaces/${workspace.id}/issues/ENG-1`,
        {
          headers,
          data: {
            expected_version: current.version,
            description: formattedDescription,
          },
        },
      )
    ).ok(),
  ).toBe(true);
  await page.reload();
  await expect(
    content.getByRole('heading', { name: 'What to build' }),
  ).toBeVisible();
  await expect(content.getByRole('table')).toBeVisible();
  const toolbar = page.getByRole('toolbar', { name: 'Description formatting' });
  await expect(toolbar).not.toBeVisible();
  const selectHeading = async (selector = 'h2') => {
    const heading = description.locator(selector).first();
    await heading.click();
    await heading.evaluate((element) => {
      const range = document.createRange();
      range.selectNodeContents(element);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    });
    await expect(toolbar).toBeVisible();
  };
  const formattingModifier = await page.evaluate(() =>
    /Mac|iP(hone|[oa]d)/.test(navigator.platform) ? 'Meta' : 'Control',
  );
  for (const modifier of ['Control', 'Meta']) {
    await selectHeading();
    await page.keyboard.press(`${modifier}+b`);
    if (test.info().project.name === 'desktop') {
      await expect(
        page.locator('[data-slot="sidebar"][data-state]'),
      ).toHaveAttribute('data-state', 'expanded');
    } else {
      await expect(page.getByRole('dialog')).toHaveCount(0);
    }
    if (modifier === formattingModifier) {
      await expect(description.locator('h2 strong')).toHaveText(
        'What to build',
      );
    }
  }
  await expect(
    toolbar.getByRole('button', { name: 'Bold', exact: true }),
  ).toHaveAttribute('aria-pressed', 'true');
  await toolbar.getByRole('button', { name: 'Bold', exact: true }).click();
  await expect(description.locator('h2 strong')).toHaveCount(0);
  await toolbar.getByRole('button', { name: 'Bold', exact: true }).click();
  await expect(description.locator('h2 strong')).toHaveText('What to build');
  for (const [label, tag] of [
    ['Italic', 'em'],
    ['Strikethrough', 's'],
    ['Inline code', 'code'],
  ]) {
    await toolbar.getByRole('button', { name: label, exact: true }).click();
    await expect(description.locator(`h2 ${tag}`)).toHaveText('What to build');
    await toolbar.getByRole('button', { name: label, exact: true }).click();
    await expect(description.locator(`h2 ${tag}`)).toHaveCount(0);
  }
  await toolbar.getByRole('button', { name: 'Bold', exact: true }).click();
  await expect(description.locator('h2 strong')).toHaveText('What to build');
  await toolbar
    .getByRole('button', { name: 'Text style', exact: true })
    .click();
  await page
    .getByRole('menuitemradio', { name: 'Heading 1', exact: true })
    .click();
  await expect(description.locator('h1')).toHaveText('What to build');
  await toolbar
    .getByRole('button', { name: 'Text style', exact: true })
    .click();
  await page
    .getByRole('menuitemradio', { name: 'Heading 2', exact: true })
    .click();
  await expect(description.locator('h2')).toHaveText('What to build');
  await toolbar.getByRole('button', { name: 'Quote', exact: true }).click();
  await expect(description.locator('blockquote h2')).toHaveText(
    'What to build',
  );
  await toolbar.getByRole('button', { name: 'Quote', exact: true }).click();
  await expect(description.locator('blockquote h2')).toHaveCount(0);
  await toolbar.getByRole('button', { name: 'Link', exact: true }).click();
  const linkInput = page.getByRole('textbox', {
    name: 'Link URL',
    exact: true,
  });
  await linkInput.fill('javascript:alert(1)');
  await page.getByRole('button', { name: 'Apply link', exact: true }).click();
  await expect(
    page.getByText('Enter a valid link, such as https://example.com.'),
  ).toBeVisible();
  await linkInput.fill('https://example.com/selected');
  await page.getByRole('button', { name: 'Apply link', exact: true }).click();
  await expect(description.locator('h2 a')).toHaveAttribute(
    'href',
    'https://example.com/selected',
  );
  await saved();
  await selectHeading('p');
  for (const colorScheme of themes) {
    await page.emulateMedia({ colorScheme });
    const line = await description
      .locator('p')
      .first()
      .evaluate((element) => {
        const range = document.createRange();
        range.selectNodeContents(element);
        const rects = [...range.getClientRects()].filter(
          (rect) => rect.width && rect.height,
        );
        const first = rects[0];
        if (!first) throw new Error('Selected text has no visible line.');
        const row = rects.filter((rect) => Math.abs(rect.top - first.top) < 3);
        return {
          top: first.top,
          center:
            (Math.min(...row.map((rect) => rect.left)) +
              Math.max(...row.map((rect) => rect.right))) /
            2,
        };
      });
    // Wait for the selection-position debounce, and catch wrong anchors or clipping.
    await expect
      .poll(async () => {
        const bounds = await toolbar.boundingBox();
        if (!bounds) return false;
        const expectedLeft = Math.max(
          12,
          Math.min(
            line.center - bounds.width / 2,
            page.viewportSize()!.width - 12 - bounds.width,
          ),
        );
        return (
          Math.abs(bounds.x - expectedLeft) < 3 &&
          Math.abs(bounds.y + bounds.height - (line.top - 8)) < 3
        );
      })
      .toBe(true);
    await page.screenshot({
      path: `test-results/issue-toolbar-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
    });
    const bounds = await toolbar.boundingBox();
    expect(bounds).not.toBeNull();
    expect(bounds!.x).toBeGreaterThanOrEqual(0);
    expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(
      page.viewportSize()!.width,
    );
  }
  await page.keyboard.press('Alt+F10');
  await expect(
    toolbar.getByRole('button', { name: 'Text style', exact: true }),
  ).toBeFocused();
  await page.keyboard.press('ArrowRight');
  await expect(
    toolbar.getByRole('button', { name: 'Bold', exact: true }),
  ).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(toolbar).not.toBeVisible();
  const task = description.locator('li[data-type="taskItem"]').first();
  const checkboxBounds = await task.getByRole('checkbox').boundingBox();
  const textBounds = await task.locator('p').first().boundingBox();
  expect(checkboxBounds).not.toBeNull();
  expect(textBounds).not.toBeNull();
  expect(Math.abs(checkboxBounds!.y - textBounds!.y)).toBeLessThan(16);
  await expect(content.getByRole('checkbox').first()).toBeChecked();
  await content.getByRole('checkbox').nth(1).check();
  await saved();
  await expect(content.getByRole('checkbox').nth(1)).toBeChecked();
  // Keyboard formatting edits the document and round-trips as Markdown.
  await description.locator('h2').click();
  await page.keyboard.press('End');
  await page.keyboard.type(' in place');
  await saved();
  await expect(
    content.getByRole('heading', { name: 'What to build in place' }),
  ).toBeVisible();
  // A delayed response must not replace text entered after the request started.
  let release: (() => void) | undefined;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  let delayed = false;
  await page.route('**/api/v1/workspaces/*/issues/*', async (route) => {
    if (route.request().method() === 'PATCH' && !delayed) {
      delayed = true;
      const response = await route.fetch();
      await gate;
      await route.fulfill({ response });
    } else await route.continue();
  });
  await page
    .getByRole('textbox', { name: 'Title', exact: true })
    .fill('Deliver the issue workflow safely');
  await expect.poll(() => delayed).toBe(true);
  await description.locator('h2').click();
  await page.keyboard.press('End');
  await page.keyboard.type(' safely');
  release?.();
  await saved();
  await page.unroute('**/api/v1/workspaces/*/issues/*');
  await expect(
    content.getByRole('heading', { name: 'What to build in place safely' }),
  ).toBeVisible();
  await page
    .getByRole('textbox', { name: 'Title', exact: true })
    .fill('Deliver the issue workflow');
  await saved();
  await page.reload();
  await expect(
    content.getByRole('heading', { name: 'What to build in place safely' }),
  ).toBeVisible();
  await expect(description.locator('h2 strong').first()).toBeVisible();
  await expect(description.locator('h2 a')).toHaveAttribute(
    'href',
    'https://example.com/selected',
  );
  await expect(content.getByRole('table')).toBeVisible();
  for (const colorScheme of themes) {
    await page.emulateMedia({ colorScheme });
    await page.screenshot({
      path: `test-results/issue-inline-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
    });
  }
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page
    .getByRole('button', { name: 'Archive issue', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Restore issue', exact: true }),
  ).toBeVisible();
  await expect(description).toHaveAttribute('contenteditable', 'false');
  await expect(toolbar).toHaveCount(0);
  await expect(
    page.getByRole('button', { name: 'Status: Done', exact: true }),
  ).toBeDisabled();
  await page.getByRole('link', { name: 'Back to issues' }).click();
  await expect(page.getByText('Capture your first issue')).toBeVisible();
  await page.getByRole('link', { name: 'Archived', exact: true }).click();
  await page.reload();
  await page
    .getByRole('link', { name: /ENG-1 Deliver the issue workflow/ })
    .click();
  await page
    .getByRole('button', { name: 'Restore issue', exact: true })
    .click();
  await expect(description).toHaveAttribute('contenteditable', 'true');
  await page.goto(`/${slug}/issue/eng-1`);
  await expect(page).toHaveURL(
    new RegExp(`/${slug}/issue/ENG-1/deliver-the-issue-workflow$`),
  );
  await page.getByRole('link', { name: 'Back to issues' }).click();
  await expect(
    page.getByText('Deliver the issue workflow', { exact: true }),
  ).toBeVisible();
  const persisted = await (
    await page.request.get(
      `/api/v1/workspaces/${workspace.id}/issues/${issueId}`,
    )
  ).json();
  expect(persisted.archived_at).toBeNull();
  expect(persisted.assignee_membership_id).toBeNull();
  const detail: components['schemas']['WorkspaceDetail'] = await (
    await page.request.get(`/api/v1/workspaces/${workspace.id}`)
  ).json();
  const team = detail.teams[0];
  const assignee = detail.members[1];
  if (!team || !assignee)
    throw new Error('Issue test requires its seeded team and second member.');
  for (const [title, category, priority] of [
    ['Add keyboard shortcuts to the editor', 'backlog', 'medium'],
    ['Document the self-hosted setup', 'backlog', 'low'],
    ['Check issue links after team rename', 'unstarted', 'high'],
    ['Preserve drafts when a request fails', 'started', 'urgent'],
    ['Improve empty states on mobile', 'started', 'none'],
    ['Keep Markdown descriptions through reload', 'completed', 'none'],
  ]) {
    const created = await page.request.post(
      `/api/v1/workspaces/${workspace.id}/issues`,
      {
        headers,
        data: {
          team_id: team.id,
          title,
          priority,
          status_id: team.statuses.find(
            (status) => status.category === category,
          )?.id,
          assignee_membership_id: assignee.id,
        },
      },
    );
    expect(created.ok()).toBe(true);
  }
  await page.reload();
  await expect(
    page.getByText('Add keyboard shortcuts to the editor'),
  ).toBeVisible();
  const backlog = page.getByRole('region', {
    name: 'ENG Backlog',
    exact: true,
  });
  await backlog.getByRole('button', { name: /Backlog 2/ }).click();
  await expect(
    page.getByText('Add keyboard shortcuts to the editor'),
  ).toBeHidden();
  await backlog.getByRole('button', { name: /Backlog 2/ }).click();
  for (const colorScheme of themes) {
    await page.emulateMedia({ colorScheme });
    await page.screenshot({
      path: `test-results/issue-list-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
    });
  }
  await page.getByRole('button', { name: 'New issue', exact: true }).click();
  await expect(page.getByLabel('Title', { exact: true })).toBeFocused();
  await page
    .getByLabel('Title', { exact: true })
    .fill('Keep creating without leaving the list');
  await page.getByLabel('Create more', { exact: true }).check();
  await page.getByRole('button', { name: 'Create issue', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'New issue', exact: true });
  await expect(dialog.getByRole('status')).toContainText('created');
  await expect(page.getByLabel('Title', { exact: true })).toHaveValue('');
  await page.getByLabel('Title', { exact: true }).fill('My next issue draft');
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  await page.getByRole('button', { name: 'New issue', exact: true }).click();
  await expect(page.getByLabel('Title', { exact: true })).toHaveValue(
    'My next issue draft',
  );
  await page.route('**/api/v1/workspaces/*/issues', (route) =>
    route.request().method() === 'POST' ? route.abort() : route.continue(),
  );
  await page.getByRole('button', { name: 'Create issue', exact: true }).click();
  await expect(dialog.getByRole('alert')).toContainText(
    'Check your connection',
  );
  await expect(page.getByLabel('Title', { exact: true })).toHaveValue(
    'My next issue draft',
  );
  await page.unroute('**/api/v1/workspaces/*/issues');
  await page
    .getByLabel('Description', { exact: true })
    .fill('Add the details your team needs.');
  for (const colorScheme of themes) {
    await page.emulateMedia({ colorScheme });
    await page.screenshot({
      path: `test-results/issue-create-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
    });
  }
  await context.close();
  await page.keyboard.press('Escape');
  // Each team has its own navigation and persisted issue-list scope.
  expect(
    (
      await page.request.post(`/api/v1/workspaces/${workspace.id}/teams`, {
        headers,
        data: { name: 'Design', key: 'DES' },
      })
    ).ok(),
  ).toBe(true);
  await page.reload();
  const navigation = page.getByRole('navigation', {
    name: 'Team navigation',
    exact: true,
  });
  async function openNavigation() {
    if (
      (page.viewportSize()?.width ?? 0) < 768 &&
      !(await navigation.isVisible())
    )
      await page.getByRole('button', { name: 'Toggle navigation' }).click();
    await expect(navigation).toBeVisible();
  }
  await openNavigation();
  const designMenu = navigation
    .getByRole('listitem')
    .filter({ has: page.getByRole('button', { name: 'Design', exact: true }) });
  const engineeringMenu = navigation.getByRole('listitem').filter({
    has: page.getByRole('button', { name: 'Engineering', exact: true }),
  });
  await expect(
    designMenu.getByRole('link', { name: 'Issues', exact: true }),
  ).toBeVisible();
  await designMenu.getByRole('button', { name: 'Design', exact: true }).click();
  await expect(
    designMenu.getByRole('link', { name: 'Issues', exact: true }),
  ).toBeHidden();
  await designMenu.getByRole('button', { name: 'Design', exact: true }).click();
  await designMenu.getByRole('link', { name: 'Home', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Design', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Workflow', exact: true }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Open issues', exact: true }).click();
  await expect(page.getByText('Capture your first issue')).toBeVisible();
  await expect(
    page.getByText('Deliver the issue workflow', { exact: true }),
  ).toHaveCount(0);
  await page.getByRole('button', { name: 'New issue', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Team: DES', exact: true }),
  ).toBeVisible();
  await page.getByLabel('Title', { exact: true }).fill('A design team issue');
  await page.getByRole('button', { name: 'Create issue', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'A design team issue', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('textbox', { name: 'Description', exact: true }),
  ).toBeEmpty();
  await page.reload();
  await openNavigation();
  await expect(
    designMenu.getByRole('link', { name: 'Issues', exact: true }),
  ).toHaveAttribute('aria-current', 'page');
  if ((page.viewportSize()?.width ?? 0) < 768)
    await page.getByRole('button', { name: 'Close navigation' }).click();
  await page
    .getByRole('button', { name: 'Archive issue', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Restore issue', exact: true }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Back to issues' }).click();
  await expect(page.getByText('Capture your first issue')).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/${slug}/team/des/all$`));
  await page.getByRole('link', { name: 'Archived', exact: true }).click();
  await page.reload();
  await expect(
    page.getByText('A design team issue', { exact: true }),
  ).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/${slug}/team/des/archive$`));
  await openNavigation();
  await designMenu
    .getByRole('link', { name: 'Issues', exact: true })
    .scrollIntoViewIfNeeded();
  for (const colorScheme of themes) {
    await page.emulateMedia({ colorScheme });
    await page.screenshot({
      path: `test-results/team-navigation-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
    });
  }
  await engineeringMenu
    .getByRole('link', { name: 'Issues', exact: true })
    .click();
  await expect(
    page.getByText('Deliver the issue workflow', { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText('A design team issue', { exact: true }),
  ).toHaveCount(0);
  await page.goto(`/${slug}/team/unknown/all`);
  await expect(page.getByRole('alert')).toContainText(
    'This team is unavailable',
  );
  await page.goto(issueUrl);
  await expect(page).toHaveURL(
    new RegExp(`/${slug}/issue/ENG-1/deliver-the-issue-workflow$`),
  );
  await expect(
    page.getByRole('heading', {
      name: 'Deliver the issue workflow',
      exact: true,
    }),
  ).toBeVisible();
  for (const path of [
    `/${slug}/issues`,
    `/${slug}/issues/${issueId}`,
    `/${slug}/team/des/issue/${issueId}`,
    `/${slug}/issue/${issueId}`,
  ]) {
    await page.goto(path);
    await expect(
      page.getByRole('heading', { name: 'Page not found', exact: true }),
    ).toBeVisible();
    await expect(page.getByLabel('Title', { exact: true })).toHaveCount(0);
  }
  await page.goto(`/${slug}/team/des/new`);
  await expect(
    page.getByRole('dialog', { name: 'New issue', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Team: DES', exact: true }),
  ).toBeVisible();
  await page.keyboard.press('Escape');
  for (const { title, titleSlug } of [
    { title: 'Café / 日本語 & C++', titleSlug: 'cafe-日本語-c' },
    { title: '!!!', titleSlug: 'issue' },
  ]) {
    const created = await page.request.post(
      `/api/v1/workspaces/${workspace.id}/issues`,
      { headers, data: { team_id: team.id, title } },
    );
    expect(created.ok()).toBe(true);
    const issue: components['schemas']['Issue'] = await created.json();
    await page.goto(`/${slug}/issue/${issue.identifier}`);
    await expect(page).toHaveURL(
      `http://127.0.0.1:5174/${slug}/issue/${issue.identifier}/${encodeURIComponent(titleSlug)}`,
    );
    await expect(
      page.getByRole('heading', { name: title, exact: true }),
    ).toBeVisible();
  }
});
