import { expect, test, type Page } from '@playwright/test';
import { webOrigin } from './environment';
const headers = { Origin: webOrigin, 'X-Convex-CSRF': '1' };
const password = 'a secure browser test password';
async function providerLogin(page: Page, username: string) {
  await expect(page.locator('#password')).toBeVisible();
  const identity = page.locator('#username');
  if ((await identity.isVisible()) && (await identity.isEditable())) {
    await identity.fill(username);
  }
  await page.locator('#password').fill('convex provider test password');
  await page.getByRole('button', { name: 'Sign In', exact: true }).click();
}
test('link, remove and re-add a password with fresh provider proofs and preserve the account', async ({
  page,
  browser,
}) => {
  test.skip(
    process.env.CONVEX_E2E_OIDC !== '1' ||
      process.env.CONVEX_E2E_OIDC_REQUIRED !== 'false',
    'Run mise run test:e2e:methods',
  );
  test.setTimeout(120_000);
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
  const device = test.info().project.name;
  const username =
    device === 'mobile' ? 'invited-member-mobile' : 'invited-member';
  const bootstrap = await (
    await page.request.get('/api/v1/auth/bootstrap')
  ).json();
  if (bootstrap.setup_required) {
    expect(
      (
        await page.request.post('/api/v1/auth/setup', {
          headers,
          data: {
            setup_token: 'browser-test-installer-credential-12345',
            email: 'browser-owner@example.com',
            password,
            display_name: 'Owner',
            workspace_name: 'Methods',
            workspace_slug: 'methods',
            team_name: 'Engineering',
            team_key: 'ENG',
          },
        })
      ).status(),
    ).toBe(200);
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
  const invitation = await (
    await page.request.post(
      `/api/v1/workspaces/${owner.workspaces[0].id}/invitations`,
      { headers, data: { role: 'member' } },
    )
  ).json();
  const context = await browser.newContext({
    baseURL: webOrigin,
    viewport: page.viewportSize() ?? undefined,
  });
  const member = await context.newPage();
  const email = `methods-${device}@example.com`;
  expect(
    (
      await member.request.post(`${webOrigin}/api/v1/invitations/accept`, {
        headers,
        data: {
          token: new URL(invitation.url).hash.slice(1),
          email,
          password,
          display_name: 'Member',
        },
      })
    ).status(),
  ).toBe(200);
  const original = await (
    await member.request.get(`${webOrigin}/api/v1/auth/me`)
  ).json();
  await member.goto('/account');
  await member
    .getByLabel('Current password to link single sign-on')
    .fill(password);
  await member
    .getByRole('button', { name: 'Link Development SSO', exact: true })
    .focus();
  await member.keyboard.press('Enter');
  await providerLogin(member, username);
  await expect(member).toHaveURL(`${webOrigin}/account?method=linked`);
  await expect(
    member.getByRole('button', {
      name: 'Verify single sign-on to remove password',
      exact: true,
    }),
  ).toBeVisible();
  await member.screenshot({
    path: `test-results/methods-linked-${device}.png`,
    fullPage: true,
  });
  await member
    .getByRole('button', {
      name: 'Verify single sign-on to remove password',
      exact: true,
    })
    .click();
  // max_age=0 must show provider authentication even though its session remains active.
  await providerLogin(member, username);
  await expect(member).toHaveURL(`${webOrigin}/account?method=proof`);
  await member
    .getByRole('button', { name: 'Remove password', exact: true })
    .click();
  await expect(
    member.getByRole('button', {
      name: 'Verify single sign-on to add a password',
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    member.getByRole('heading', { name: 'Change password', exact: true }),
  ).toHaveCount(0);
  await member
    .getByRole('button', {
      name: 'Verify single sign-on to add a password',
      exact: true,
    })
    .click();
  await providerLogin(member, username);
  await member.getByLabel('Email for password login').fill(email);
  await member.getByLabel('New password', { exact: true }).fill(password);
  await member
    .getByRole('button', { name: 'Add password', exact: true })
    .click();
  await expect(
    member.getByRole('heading', { name: 'Change password', exact: true }),
  ).toBeVisible();
  const account = await (await member.request.get('/api/v1/auth/me')).json();
  expect(account.user.id).toBe(original.user.id);
  expect(account.workspaces).toEqual(original.workspaces);
  await member
    .getByLabel('Current password to remove single sign-on')
    .fill(password);
  await member
    .getByRole('button', { name: 'Remove single sign-on', exact: true })
    .click();
  await expect(
    member.getByRole('button', { name: 'Link Development SSO', exact: true }),
  ).toBeVisible();
  await member.screenshot({
    path: `test-results/methods-local-${device}.png`,
    fullPage: true,
  });
  await context.close();
});
