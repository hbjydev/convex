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
    page.getByRole('heading', { name: 'ENG-1', exact: true }),
  ).toBeVisible();
  const issueUrl = page.url();
  await expect(page).toHaveURL(
    new RegExp(`/${slug}/issue/ENG-1/ship-the-issue-workflow$`),
  );
  const initialIssue: components['schemas']['Issue'] = await (
    await page.request.get(`/api/v1/workspaces/${workspace.id}/issues/ENG-1`)
  ).json();
  const issueId = initialIssue.id;
  await second.goto(issueUrl);
  await expect(second.getByLabel('Title', { exact: true })).toHaveValue(
    'Ship the issue workflow',
  );
  await page
    .getByLabel('Title', { exact: true })
    .fill('Deliver the issue workflow');
  await second.getByLabel('Priority', { exact: true }).selectOption('high');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Changes saved');
  await expect(page).toHaveURL(
    new RegExp(`/${slug}/issue/ENG-1/deliver-the-issue-workflow$`),
  );
  await second
    .getByRole('button', { name: 'Save changes', exact: true })
    .click();
  await expect(second.getByRole('alert')).toContainText(
    'Your draft is preserved',
  );
  await expect(second.getByLabel('Priority', { exact: true })).toHaveValue(
    'high',
  );
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
  await expect(second.getByLabel('Title', { exact: true })).toHaveValue(
    'Deliver the issue workflow',
  );
  await second
    .getByRole('button', { name: 'Save changes', exact: true })
    .click();
  await expect(second.getByRole('status')).toContainText('Changes saved');
  // Same-property conflict requires an explicit choice, and adopting current data works.
  await page.getByLabel('Title', { exact: true }).fill('My alternate title');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText(
    'Your draft is preserved',
  );
  await expect(page.getByLabel('Title', { exact: true })).toHaveValue(
    'My alternate title',
  );
  await page.getByRole('button', { name: 'Use current version' }).click();
  await expect(page.getByLabel('Title', { exact: true })).toHaveValue(
    'Deliver the issue workflow',
  );
  // A failed request retains the editor draft and can be retried.
  await page.route('**/api/v1/workspaces/*/issues/*', (route) =>
    route.request().method() === 'PATCH' ? route.abort() : route.continue(),
  );
  await page
    .getByLabel('Description', { exact: true })
    .fill('## Final\nPersist this description.');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Check your connection');
  await expect(page.getByLabel('Description', { exact: true })).toHaveValue(
    '## Final\nPersist this description.',
  );
  await page.unroute('**/api/v1/workspaces/*/issues/*');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Changes saved');
  await page.reload();
  await expect(page.getByLabel('Description', { exact: true })).toHaveValue(
    '## Final\nPersist this description.',
  );
  await expect(page.getByLabel('Priority', { exact: true })).toHaveValue(
    'high',
  );
  await page.getByLabel('Assignee', { exact: true }).selectOption('');
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Changes saved');
  await page
    .getByLabel('Status', { exact: true })
    .selectOption({ label: 'Done' });
  await page.getByRole('button', { name: 'Save changes', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Changes saved');
  await page.screenshot({
    path: `test-results/issue-detail-${test.info().project.name}.png`,
    fullPage: true,
  });
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
  await expect(page.getByLabel('Title', { exact: true })).toBeDisabled();
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
  await expect(page.getByLabel('Title', { exact: true })).toBeEnabled();
  await page.goto(`/${slug}/issue/eng-1`);
  await expect(page).toHaveURL(
    new RegExp(`/${slug}/issue/ENG-1/deliver-the-issue-workflow$`),
  );
  await expect(page.getByLabel('Title', { exact: true })).toHaveValue(
    'Deliver the issue workflow',
  );
  await page.getByRole('link', { name: 'Back to issues' }).click();
  await expect(
    page.getByText('Deliver the issue workflow', { exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: `test-results/issue-list-${test.info().project.name}.png`,
    fullPage: true,
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
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
    page.getByRole('heading', { name: 'DES-1', exact: true }),
  ).toBeVisible();
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
  await expect(page.getByLabel('Title', { exact: true })).toHaveValue(
    'Deliver the issue workflow',
  );
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
    await expect(page.getByLabel('Title', { exact: true })).toHaveValue(title);
  }
});
