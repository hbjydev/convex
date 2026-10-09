import { expect, test } from '@playwright/test';
import { webOrigin } from './environment';

const headers = { Origin: webOrigin, 'X-Convex-CSRF': '1' };
const password = 'a secure browser test password';
test('required OIDC invitation, email isolation, session login and sanitized callback failure', async ({
  page,
  browser,
}) => {
  test.skip(process.env.CONVEX_E2E_OIDC !== '1', 'Run mise run test:e2e:oidc');
  test.setTimeout(90_000);
  await expect
    .poll(
      async () => {
        try {
          return (
            await page.request.get(
              'http://127.0.0.1:8083/realms/convex/.well-known/openid-configuration',
            )
          ).status();
        } catch {
          return 0;
        }
      },
      { timeout: 60_000 },
    )
    .toBe(200);
  const bootstrap = await page.request.get('/api/v1/auth/bootstrap');
  if ((await bootstrap.json()).setup_required) {
    const response = await page.request.post('/api/v1/auth/setup', {
      headers,
      data: {
        setup_token: 'browser-test-installer-credential-12345',
        email: 'browser-owner@example.com',
        password,
        display_name: 'Local owner',
        workspace_name: 'OIDC workspace',
        workspace_slug: 'oidc-workspace',
        team_name: 'Engineering',
        team_key: 'ENG',
      },
    });
    expect(response.status()).toBe(200);
  } else {
    expect(
      (
        await page.request.post('/api/v1/auth/login', {
          headers,
          data: { email: 'browser-owner@example.com', password },
        })
      ).status(),
    ).toBe(200);
  }
  const owner = await (await page.request.get('/api/v1/auth/me')).json();
  const workspaceResponse = await page.request.post('/api/v1/workspaces', {
    headers,
    data: {
      name: `OIDC ${test.info().project.name}`,
      slug: `oidc-${test.info().project.name}`,
      team_name: 'Engineering',
      team_key: 'ENG',
    },
  });
  expect(workspaceResponse.status()).toBe(200);
  const workspace = await workspaceResponse.json();
  const invitation = await (
    await page.request.post(`/api/v1/workspaces/${workspace.id}/invitations`, {
      headers,
      data: { role: 'member' },
    })
  ).json();
  const context = await browser.newContext({
    viewport: page.viewportSize() ?? undefined,
  });
  const member = await context.newPage();
  await member.goto(invitation.url);
  await expect(
    member.getByRole('button', { name: 'Continue with Development SSO' }),
  ).toBeVisible();
  await expect(member.getByLabel('Email', { exact: true })).toHaveCount(0);
  await member.screenshot({
    path: `test-results/oidc-invite-${test.info().project.name}.png`,
    fullPage: true,
  });
  await member
    .getByRole('button', { name: 'Continue with Development SSO' })
    .click();
  await member.locator('#username').fill('invited-member');
  await member.locator('#password').fill('convex provider test password');
  await member.getByRole('button', { name: 'Sign In', exact: true }).click();
  await expect(member).toHaveURL(new RegExp(`${webOrigin}/$`));
  const account = await (await member.request.get('/api/v1/auth/me')).json();
  expect(account.user.id).not.toBe(owner.user.id);
  expect(account.user.display_name).toBe('OIDC Member');
  expect(account.workspaces.map((space: { id: string }) => space.id)).toContain(
    workspace.id,
  );
  expect(account.user.is_instance_admin).toBe(false);
  // Equal provider/local emails remain independent accounts; repeating this
  // flow on mobile uses the same attached provider identity and membership.
  expect(
    (await member.request.post('/api/v1/auth/logout', { headers })).status(),
  ).toBe(200);
  await member.goto('/auth/login');
  await expect(
    member.getByRole('button', { name: 'Administrator sign-in', exact: true }),
  ).toBeVisible();
  await member.screenshot({
    path: `test-results/oidc-login-${test.info().project.name}.png`,
    fullPage: true,
  });
  await member
    .getByRole('button', { name: 'Continue with Development SSO' })
    .focus();
  await member.keyboard.press('Enter');
  await expect(member).toHaveURL(new RegExp(`${webOrigin}/$`));
  expect(
    (await (await member.request.get('/api/v1/auth/me')).json()).user.id,
  ).toBe(account.user.id);
  await member.goto('/api/v1/auth/oidc/callback?state=bad&code=secret-code');
  await expect(member).toHaveURL(`${webOrigin}/auth/oidc?outcome=failed`);
  await expect(
    member.getByRole('heading', { name: 'Sign-in did not complete.' }),
  ).toBeVisible();
  await member.screenshot({
    path: `test-results/oidc-failure-${test.info().project.name}.png`,
    fullPage: true,
  });
  await context.close();
});
