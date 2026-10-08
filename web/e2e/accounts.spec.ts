import { execFile } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { promisify } from 'node:util';
import { expect, test } from '@playwright/test';

const password = 'a secure browser test password';
const newPassword = 'a replacement browser password';
test('two people onboard, sign in, manage sessions, and lose removed access', async ({
  page,
  browser,
}) => {
  test.setTimeout(60_000);
  const suffix = `${test.info().project.name}-${Date.now()}`;
  await page.goto('/');
  const bootstrap = await page.request.get('/api/v1/auth/bootstrap');
  if ((await bootstrap.json()).setup_required) {
    await expect(
      page.getByRole('heading', { name: 'Set up Convex' }),
    ).toBeVisible();
    await page.screenshot({
      path: `test-results/setup-${test.info().project.name}.png`,
      fullPage: true,
    });
    const viewport = page.viewportSize();
    await page.setViewportSize({ width: 393, height: 851 });
    await page.screenshot({
      path: 'test-results/setup-mobile.png',
      fullPage: true,
    });
    if (viewport) await page.setViewportSize(viewport);
    await page
      .getByLabel('Installation setup credential')
      .fill('browser-test-installer-credential-12345');
    await page
      .getByLabel('Your name', { exact: true })
      .fill('Installation owner');
    await page
      .getByLabel('Email', { exact: true })
      .fill('browser-owner@example.com');
    await page.getByLabel('Password', { exact: true }).fill(password);
    await page
      .getByLabel('Workspace name', { exact: true })
      .fill('Initial workspace');
    await page.getByLabel('First team name').fill('Engineering');
    await page.getByLabel('Team key', { exact: true }).fill('ENG');
    await page.getByRole('button', { name: 'Create workspace' }).click();
    await expect(page).toHaveURL(/\/initial-workspace$/);
    await page.goto('/');
  } else {
    await page
      .getByLabel('Email', { exact: true })
      .fill('browser-owner@example.com');
    await page.getByLabel('Password', { exact: true }).fill(password);
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  }
  await expect(
    page.getByRole('heading', { name: 'Your workspaces', exact: true }),
  ).toBeVisible();
  await page
    .getByLabel('Workspace name', { exact: true })
    .fill(`Workspace ${suffix}`);
  await page.getByLabel('First team name').fill('Product');
  await page.getByLabel('Team key', { exact: true }).fill('PROD');
  await page.getByRole('button', { name: 'Create workspace' }).click();
  await expect(
    page.getByRole('heading', { name: 'Workspace', exact: true }),
  ).toBeVisible();
  const workspaceUrl = page.url();
  const workspacePath = new URL(workspaceUrl).pathname;
  await page.goto(`${workspaceUrl}#people`);
  await page.getByRole('button', { name: 'Invite', exact: true }).click();
  await page.getByRole('button', { name: 'Create invitation' }).click();
  const link = page.getByLabel('Single-use invitation link');
  await expect(link).toBeVisible();
  const inviteUrl = await link.inputValue();
  const teammate = await browser.newContext({
    viewport: page.viewportSize() ?? undefined,
  });
  const member = await teammate.newPage();
  await member.goto(inviteUrl);
  await expect(
    member.getByRole('heading', { name: `Join Workspace ${suffix}` }),
  ).toBeVisible();
  await member.screenshot({
    path: `test-results/invitation-${test.info().project.name}.png`,
    fullPage: true,
  });
  await member.getByLabel('Your name', { exact: true }).fill('Second person');
  await member
    .getByLabel('Email', { exact: true })
    .fill(`${suffix}@example.com`);
  await member.getByLabel('Password', { exact: true }).fill(password);
  await member.getByRole('button', { name: 'Join workspace' }).click();
  await expect(
    member.getByRole('heading', { name: 'Workspace', exact: true }),
  ).toBeVisible();
  await expect(
    member.getByRole('button', { name: 'Create invitation' }),
  ).toHaveCount(0);
  await page.reload();
  await expect(page.getByText('Second person', { exact: true })).toBeVisible();
  await page.screenshot({
    path: `test-results/workspace-${test.info().project.name}.png`,
    fullPage: true,
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  // A replay cannot onboard a third person.
  const replay = await browser.newContext();
  const third = await replay.newPage();
  await third.goto(inviteUrl);
  await expect(third.getByRole('alert')).toContainText('already used');
  await replay.close();
  if ((page.viewportSize()?.width ?? 0) < 768)
    await member.getByRole('button', { name: 'Toggle navigation' }).click();
  await member.getByRole('link', { name: 'Your account', exact: true }).click();
  await member.screenshot({
    path: `test-results/account-${test.info().project.name}.png`,
    fullPage: true,
  });
  await member.getByLabel('Current password', { exact: true }).fill(password);
  await member.getByLabel('New password', { exact: true }).fill(newPassword);
  await member.getByRole('button', { name: 'Change password' }).click();
  await expect(member.getByRole('status')).toContainText('Password changed');
  if ((page.viewportSize()?.width ?? 0) < 768)
    await member.getByRole('button', { name: 'Toggle navigation' }).click();
  await member
    .getByRole('button', { name: `Workspace ${suffix} workspace menu` })
    .click();
  await member.getByRole('menuitem', { name: 'Sign out', exact: true }).click();
  await member.goto('/auth/login');
  await member
    .getByLabel('Email', { exact: true })
    .fill(`${suffix}@example.com`);
  await member.getByLabel('Password', { exact: true }).fill(newPassword);
  await member.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(
    member.getByRole('heading', { name: 'Your workspaces', exact: true }),
  ).toBeVisible();
  const account = await (await page.request.get('/api/v1/auth/me')).json();
  const workspaceId = account.workspaces.find(
    (workspace: { slug: string }) => `/${workspace.slug}` === workspacePath,
  ).id;
  const workspaceResponse = await page.request.get(
    `/api/v1/workspaces/${workspaceId}`,
  );
  const memberId = (await workspaceResponse.json()).members.find(
    (m: { display_name: string }) => m.display_name === 'Second person',
  ).user_id;
  const row = page
    .getByRole('row')
    .filter({ has: page.getByText('Second person', { exact: true }) });
  await row.getByRole('button', { name: 'Manage Second person' }).click();
  const editor = page.getByRole('dialog', { name: 'Manage Second person' });
  await editor.getByRole('button', { name: 'Remove', exact: true }).click();
  await editor
    .getByRole('button', { name: 'Confirm removal', exact: true })
    .click();
  await expect(page.getByText('Second person', { exact: true })).toHaveCount(0);
  await member.goto(workspaceUrl);
  await expect(
    member.getByRole('heading', { name: 'Welcome back.' }),
  ).toBeVisible();
  await member.screenshot({
    path: `test-results/login-${test.info().project.name}.png`,
    fullPage: true,
  });
  await member.getByRole('link', { name: 'Forgot your password?' }).click();
  await expect(
    member.getByText('Email recovery is unavailable', { exact: false }),
  ).toBeVisible();
  await member.screenshot({
    path: `test-results/recovery-${test.info().project.name}.png`,
    fullPage: true,
  });
  // Exercise the documented operator command with mail unavailable, including explicit enablement.
  const disabled = await page.request.post('/api/v1/admin/accounts/disable', {
    headers: { Origin: 'http://127.0.0.1:5174', 'X-Convex-CSRF': '1' },
    data: { user_id: memberId, disabled: true },
  });
  expect(disabled.ok()).toBe(true);
  const database = (await readFile('../target/e2e-database', 'utf8')).trim();
  const databaseUrl = new URL(process.env.DATABASE_URL ?? '');
  databaseUrl.pathname = database;
  const { stdout } = await promisify(execFile)(
    'target/debug/recover-account',
    [`${suffix}@example.com`, '--enable'],
    {
      cwd: '..',
      env: {
        ...process.env,
        DATABASE_URL: databaseUrl.toString(),
        CONVEX_PUBLIC_URL: 'http://127.0.0.1:5174',
        CONVEX_SMTP_HOST: '',
      },
    },
  );
  const resetUrl = stdout
    .split('\n')
    .find((line) => line.startsWith('http://127.0.0.1:5174/auth/reset#'));
  expect(resetUrl).toBeTruthy();
  await member.goto(resetUrl ?? '');
  await member.getByLabel('New password', { exact: true }).fill(password);
  await member
    .getByRole('button', { name: 'Reset password', exact: true })
    .click();
  await expect(member.getByRole('status')).toContainText('password was reset');
  await member.getByRole('link', { name: 'Sign in', exact: true }).click();
  await member
    .getByLabel('Email', { exact: true })
    .fill(`${suffix}@example.com`);
  await member.getByLabel('Password', { exact: true }).fill(password);
  await member.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(
    member.getByText('You have no active workspace memberships.', {
      exact: false,
    }),
  ).toBeVisible();
  await teammate.close();
});

test('valid unbroken names fit and network errors preserve login drafts', async ({
  page,
  browser,
  request,
}) => {
  test.setTimeout(60_000);
  if (
    (
      await (
        await request.get('http://127.0.0.1:5174/api/v1/auth/bootstrap')
      ).json()
    ).setup_required
  ) {
    const setup = await request.post(
      'http://127.0.0.1:5174/api/v1/auth/setup',
      {
        headers: { Origin: 'http://127.0.0.1:5174', 'X-Convex-CSRF': '1' },
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
      },
    );
    expect(setup.ok()).toBe(true);
  }
  await page.goto('/auth/login');
  await page
    .getByLabel('Email', { exact: true })
    .fill('browser-owner@example.com');
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.route('**/api/v1/auth/login', (route) => route.abort());
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText(
    'Check your connection and try again',
  );
  await expect(page.getByLabel('Email', { exact: true })).toHaveValue(
    'browser-owner@example.com',
  );
  await expect(page.getByLabel('Password', { exact: true })).toHaveValue(
    password,
  );
  await page.screenshot({
    path: `test-results/login-error-${test.info().project.name}.png`,
    fullPage: true,
  });
  await page.unroute('**/api/v1/auth/login');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Your workspaces', exact: true }),
  ).toBeVisible();
  const name = 'W'.repeat(100);
  await page.getByLabel('Workspace name', { exact: true }).fill(name);
  await page
    .getByLabel('Workspace URL', { exact: true })
    .fill(`long-${test.info().project.name}-${Date.now()}`);
  await page.getByLabel('First team name').fill('T'.repeat(100));
  await page.getByLabel('Team key', { exact: true }).fill('LONG');
  await page
    .getByRole('button', { name: 'Create workspace', exact: true })
    .click();
  await expect(
    page.getByRole('heading', { name: 'Workspace', exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel('Workspace name', { exact: true })).toHaveValue(
    name,
  );
  await page.goto(`${page.url()}#people`);
  await page.getByRole('button', { name: 'Invite', exact: true }).click();
  await page
    .getByRole('button', { name: 'Create invitation', exact: true })
    .click();
  const link = page.getByLabel('Single-use invitation link');
  await expect(link).toBeVisible();
  const context = await browser.newContext({
    viewport: page.viewportSize() ?? undefined,
  });
  const member = await context.newPage();
  await member.goto(await link.inputValue());
  await member.getByLabel('Your name', { exact: true }).fill('N'.repeat(100));
  await member
    .getByLabel('Email', { exact: true })
    .fill(`long-${test.info().project.name}-${Date.now()}@example.com`);
  await member.getByLabel('Password', { exact: true }).fill(password);
  await member
    .getByRole('button', { name: 'Join workspace', exact: true })
    .click();
  await expect(
    member.getByRole('heading', { name: 'Workspace', exact: true }),
  ).toBeVisible();
  for (const target of [page, member])
    expect(
      await target.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
  await member.screenshot({
    path: `test-results/long-names-${test.info().project.name}.png`,
    fullPage: true,
  });
  await page.screenshot({
    path: `test-results/long-settings-${test.info().project.name}.png`,
    fullPage: true,
  });
  await context.close();
});
