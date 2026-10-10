import { execFile } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { promisify } from 'node:util';
import { expect, test, type Page } from '@playwright/test';
import { webOrigin } from './environment';

const run = promisify(execFile);
const headers = { Origin: webOrigin, 'X-Convex-CSRF': '1' };
const initialPassword = 'a secure browser test password';
const recoveredPassword = 'a recovered administrator password';
const issuer = 'http://127.0.0.1:8083/realms/convex';
async function providerReady(page: Page) {
  await expect
    .poll(
      async () => {
        try {
          return (
            await page.request.get(
              `${issuer}/.well-known/openid-configuration`,
              { timeout: 2000 },
            )
          ).status();
        } catch {
          return 0;
        }
      },
      { timeout: 60_000 },
    )
    .toBe(200);
}
async function providerLogin(page: Page, username: string) {
  await expect(page.locator('#password')).toBeVisible();
  const field = page.locator('#username');
  if ((await field.isVisible()) && (await field.isEditable()))
    await field.fill(username);
  await page.locator('#password').fill('convex provider test password');
  await page.getByRole('button', { name: 'Sign In', exact: true }).click();
}
async function holdCallback(
  page: Page,
  authorizationUrl: string,
  username: string,
) {
  let callback = '';
  // Intercept the provider's form response before its redirect reaches Convex.
  // Playwright does not route each hop in a redirected navigation.
  await page.route('**/login-actions/authenticate?**', async (route) => {
    const response = await route.fetch({ maxRedirects: 0 });
    const location = response.headers().location;
    if (location?.startsWith(`${webOrigin}/api/v1/auth/oidc/callback?`)) {
      callback = location;
      const responseHeaders = { ...response.headers() };
      delete responseHeaders.location;
      await route.fulfill({
        response,
        status: 200,
        headers: responseHeaders,
        body: 'Provider callback held for outage test.',
      });
    } else {
      await route.fulfill({ response });
    }
  });
  await page.goto(authorizationUrl);
  await providerLogin(page, username);
  await expect.poll(() => callback).not.toBe('');
  await page.unroute('**/login-actions/authenticate?**');
  return callback;
}

test('real provider outage retains sessions and enforcement, permits local operator recovery, and restores login', async ({
  page,
  browser,
}) => {
  test.skip(
    process.env.CONVEX_E2E_OUTAGE !== '1' ||
      process.env.CONVEX_E2E_OIDC !== '1' ||
      process.env.CONVEX_E2E_OIDC_REQUIRED !== 'true',
    'Run mise run test:e2e:outage exclusively; this stops the disposable provider',
  );
  test.setTimeout(180_000);
  const device = test.info().project.name;
  const username =
    device === 'mobile' ? 'invited-member-mobile' : 'invited-member';
  await providerReady(page);
  const bootstrap = await (
    await page.request.get('/api/v1/auth/bootstrap')
  ).json();
  expect(bootstrap.oidc_required).toBe(true);
  const password = bootstrap.setup_required
    ? initialPassword
    : recoveredPassword;
  const ownerEmail = 'outage-owner@example.com';
  const login = bootstrap.setup_required
    ? await page.request.post('/api/v1/auth/setup', {
        headers,
        data: {
          setup_token: 'browser-test-installer-credential-12345',
          email: ownerEmail,
          password,
          display_name: 'Outage owner',
          workspace_name: 'Outage',
          workspace_slug: 'outage',
          team_name: 'Engineering',
          team_key: 'ENG',
        },
      })
    : await page.request.post('/api/v1/auth/login', {
        headers,
        data: { email: ownerEmail, password },
      });
  expect(login.status()).toBe(200);
  const originalOwner = await (
    await page.request.get('/api/v1/auth/me')
  ).json();
  const invitation = await (
    await page.request.post(
      `/api/v1/workspaces/${originalOwner.workspaces[0].id}/invitations`,
      { headers, data: { role: 'member' } },
    )
  ).json();
  const context = await browser.newContext({
    baseURL: webOrigin,
    viewport: page.viewportSize() ?? undefined,
  });
  const guestContext = await browser.newContext({
    baseURL: webOrigin,
    viewport: page.viewportSize() ?? undefined,
  });
  const recoveryContext = await browser.newContext({
    baseURL: webOrigin,
    viewport: page.viewportSize() ?? undefined,
  });
  const member = await context.newPage();
  const guest = await guestContext.newPage();
  const recovery = await recoveryContext.newPage();
  try {
    await member.goto(invitation.url);
    await member
      .getByRole('button', { name: 'Continue with Development SSO' })
      .focus();
    await member.keyboard.press('Enter');
    await providerLogin(member, username);
    await expect(member).toHaveURL(`${webOrigin}/`);
    const original = await (await member.request.get('/api/v1/auth/me')).json();
    const sessions = await (
      await member.request.get('/api/v1/auth/sessions')
    ).json();
    // Capture real provider-issued codes, then lose the provider before exchange.
    const fresh = await (
      await guest.request.post('/api/v1/auth/oidc/start', {
        headers,
        data: { invitation_token: null },
      })
    ).json();
    const failedLogin = await holdCallback(
      guest,
      fresh.authorization_url,
      username,
    );
    const link = await (
      await page.request.post('/api/v1/auth/methods/oidc/link', {
        headers,
        data: { current_password: password },
      })
    ).json();
    const failedLink = await holdCallback(
      page,
      link.authorization_url,
      username,
    );
    await run('docker', ['compose', '--profile', 'oidc', 'stop', 'oidc-cp03'], {
      cwd: '..',
      timeout: 30_000,
    });
    await expect
      .poll(async () => {
        try {
          return (
            await member.request.get(
              `${issuer}/.well-known/openid-configuration`,
              { timeout: 2000 },
            )
          ).status();
        } catch {
          return 0;
        }
      })
      .toBe(0);
    await guest.goto(failedLogin);
    await expect(guest).toHaveURL(`${webOrigin}/auth/oidc?outcome=failed`);
    await expect(
      guest.getByRole('heading', { name: 'Sign-in did not complete.' }),
    ).toBeVisible();
    await page.goto(failedLink);
    await expect(page).toHaveURL(`${webOrigin}/auth/oidc?outcome=failed`);
    expect((await page.request.get('/api/v1/auth/methods')).status()).toBe(200);
    const ownerMethods = await (
      await page.request.get('/api/v1/auth/methods')
    ).json();
    expect(ownerMethods.oidc).toEqual([]);
    await member.goto('/account');
    await expect(
      member.getByRole('heading', { name: 'Your account' }),
    ).toBeVisible();
    expect(await (await member.request.get('/api/v1/auth/me')).json()).toEqual(
      original,
    );
    const during = await (
      await member.request.get('/api/v1/auth/sessions')
    ).json();
    expect(during.map((s: { expires_at: string }) => s.expires_at)).toEqual(
      sessions.map((s: { expires_at: string }) => s.expires_at),
    );
    expect(
      (
        await member.request.post('/api/v1/auth/methods/local/add', {
          headers,
          data: {
            email: `outage-${device}@example.com`,
            password: initialPassword,
          },
        })
      ).status(),
    ).toBe(403);
    expect(
      (
        await guest.request.post('/api/v1/auth/login', {
          headers,
          data: {
            email: `outage-${device}@example.com`,
            password: initialPassword,
          },
        })
      ).status(),
    ).toBe(401);
    // A new local administrator login and administration work without provider access.
    await recovery.goto('/auth/login');
    await recovery.getByLabel('Email', { exact: true }).fill(ownerEmail);
    await recovery.getByLabel('Password', { exact: true }).fill(password);
    await recovery
      .getByRole('button', { name: 'Administrator sign-in', exact: true })
      .focus();
    await recovery.keyboard.press('Enter');
    await expect(recovery).toHaveURL(`${webOrigin}/`);
    expect(
      (await recovery.request.get('/api/v1/admin/accounts')).status(),
    ).toBe(200);
    const database = (await readFile('../target/e2e-database', 'utf8')).trim();
    const databaseUrl = new URL(process.env.DATABASE_URL ?? '');
    databaseUrl.pathname = database;
    // Retain the required-provider environment to exercise the deployed configuration.
    const { stdout } = await run('target/debug/recover-account', [ownerEmail], {
      cwd: '..',
      timeout: 30_000,
      env: {
        ...process.env,
        DATABASE_URL: databaseUrl.toString(),
        CONVEX_PUBLIC_URL: webOrigin,
        CONVEX_SMTP_HOST: '',
        CONVEX_SESSION_TTL_SECONDS: '604800',
        CONVEX_OIDC_ISSUER_URL: issuer,
        CONVEX_OIDC_CLIENT_ID: 'convex',
        CONVEX_OIDC_CLIENT_SECRET: 'convex-development-client-secret',
        CONVEX_OIDC_DISPLAY_NAME: 'Development SSO',
        CONVEX_OIDC_ENABLED: 'true',
        CONVEX_OIDC_REQUIRED: 'true',
      },
    });
    const resetUrl = stdout
      .split('\n')
      .find((line) => line.startsWith(`${webOrigin}/auth/reset#`));
    expect(resetUrl).toBeTruthy();
    expect((await page.request.get('/api/v1/auth/me')).status()).toBe(401);
    expect((await recovery.request.get('/api/v1/auth/me')).status()).toBe(401);
    expect((await member.request.get('/api/v1/auth/me')).status()).toBe(200);
    await recovery.goto(resetUrl ?? '');
    await recovery
      .getByLabel('New password', { exact: true })
      .fill(recoveredPassword);
    await recovery
      .getByRole('button', { name: 'Reset password', exact: true })
      .focus();
    await recovery.keyboard.press('Enter');
    await expect(recovery.getByRole('status')).toContainText(
      'password was reset',
    );
    await recovery.getByRole('link', { name: 'Sign in', exact: true }).click();
    await recovery.getByLabel('Email', { exact: true }).fill(ownerEmail);
    await recovery
      .getByLabel('Password', { exact: true })
      .fill(recoveredPassword);
    await recovery
      .getByRole('button', { name: 'Administrator sign-in', exact: true })
      .click();
    await expect(recovery).toHaveURL(`${webOrigin}/`);
    expect(
      (await (await recovery.request.get('/api/v1/auth/me')).json()).user.id,
    ).toBe(originalOwner.user.id);
    expect(
      (await recovery.request.get('/api/v1/admin/accounts')).status(),
    ).toBe(200);
    await recovery.screenshot({
      path: `test-results/outage-recovery-${device}.png`,
      fullPage: true,
    });
    await run(
      'docker',
      ['compose', '--profile', 'oidc', 'start', 'oidc-cp03'],
      { cwd: '..', timeout: 30_000 },
    );
    await providerReady(member);
    // The failed exchange remains burned even after the provider returns.
    await guest.goto(failedLogin);
    await expect(guest).toHaveURL(`${webOrigin}/auth/oidc?outcome=failed`);
    await guest.goto('/auth/login');
    await guest
      .getByRole('button', { name: 'Continue with Development SSO' })
      .click();
    await expect(guest).toHaveURL(`${webOrigin}/`);
    const restored = await (await guest.request.get('/api/v1/auth/me')).json();
    expect(restored.user.id).toBe(original.user.id);
    expect(restored.workspaces).toEqual(original.workspaces);
    expect((await member.request.get('/api/v1/auth/me')).status()).toBe(200);
  } finally {
    // Restart even on assertion failure so other provider suites can run safely.
    await run(
      'docker',
      ['compose', '--profile', 'oidc', 'start', 'oidc-cp03'],
      { cwd: '..', timeout: 30_000 },
    );
    await Promise.all([
      context.close(),
      guestContext.close(),
      recoveryContext.close(),
    ]);
  }
});
