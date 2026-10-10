import { expect, type Locator, type Page, test } from '@playwright/test';
import type { components } from '../src/api/schema';

type Schema = components['schemas'];

async function mockAccount(page: Page) {
  const account: Schema['AccountResponse'] = {
    user: {
      id: 'owner',
      display_name: 'Installation owner',
      is_instance_admin: false,
    },
    workspaces: [
      { id: 'workspace', name: 'Product', slug: 'product', role: 'owner' },
    ],
  };
  await page.route('**/api/v1/auth/bootstrap', (route) =>
    route.fulfill({
      json: {
        setup_required: false,
        setup_enabled: false,
        mail_available: false,
        oidc_required: false,
        oidc_display_name: null,
      } satisfies Schema['BootstrapInfo'],
    }),
  );
  await page.route('**/api/v1/auth/methods', (route) =>
    route.fulfill({
      json: {
        local_email: 'owner@example.com',
        local_usable: true,
        oidc: [],
        provider_display_name: null,
        can_link_oidc: false,
        can_add_local: false,
        can_remove_local: false,
        pending_action: null,
      } satisfies Schema['Methods'],
    }),
  );
  await page.route('**/api/v1/auth/me', (route) =>
    route.fulfill({ json: account }),
  );
  await page.route(/\/api\/v1\/workspaces\/[^/?]+$/, (route) => {
    const id = new URL(route.request().url()).pathname.split('/').at(-1);
    const workspace = account.workspaces.find(
      (workspace) => workspace.id === id,
    );
    return workspace && route.request().method() === 'GET'
      ? route.fulfill({
          json: {
            workspace,
            members: [],
            teams: [],
          } satisfies Schema['WorkspaceDetail'],
        })
      : route.fallback();
  });
  return account;
}

async function expectInside(control: Locator, container: Locator) {
  const box = await control.boundingBox();
  const boundary = await container.boundingBox();
  expect(box).not.toBeNull();
  expect(boundary).not.toBeNull();
  if (!box || !boundary) throw new Error('Control or container is not visible');
  expect(box.x).toBeGreaterThanOrEqual(boundary.x);
  expect(box.x + box.width).toBeLessThanOrEqual(
    boundary.x + boundary.width + 1,
  );
}

async function expectNavigation(page: Page) {
  const navigation = page.getByRole('navigation', { name: 'Team navigation' });
  await expect(
    page.getByRole('navigation', { name: 'Main navigation' }),
  ).toHaveCount(0);
  await expect(
    page.getByRole('navigation', { name: 'Workspace navigation' }),
  ).toHaveCount(0);
  if ((page.viewportSize()?.width ?? 0) < 768) {
    await page.getByRole('button', { name: 'Toggle navigation' }).click();
    await expect(navigation).toBeVisible();
    await page.getByRole('button', { name: 'Close navigation' }).click();
  } else {
    await expect(navigation).toBeVisible();
  }
}

test('authenticated homepage renders inside the app shell', async ({
  page,
}) => {
  await mockAccount(page);
  await page.goto('/');
  await expect(
    page.getByRole('heading', { name: 'Your workspaces' }),
  ).toBeVisible();
  await expectNavigation(page);
  await expect(
    page.getByRole('main').getByRole('heading', { name: 'Your workspaces' }),
  ).toBeVisible();
});

test('homepage redirects to sign in and keeps the shell after login', async ({
  page,
}) => {
  const account = await mockAccount(page);
  let signedIn = false;
  await page.route('**/api/v1/auth/me', (route) =>
    signedIn
      ? route.fulfill({ json: account })
      : route.fulfill({ status: 401, json: { message: 'Sign in required' } }),
  );
  await page.route('**/api/v1/auth/login', (route) => {
    signedIn = true;
    return route.fulfill({ json: account });
  });
  await page.goto('/');
  await expect(page).toHaveURL(/\/auth\/login$/);
  await expect(
    page.getByRole('textbox', { name: 'Email', exact: true }),
  ).toBeVisible();
  await page
    .getByRole('textbox', { name: 'Email', exact: true })
    .fill('owner@example.test');
  await page.getByLabel('Password', { exact: true }).fill('a-valid-password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(
    page.getByRole('main').getByRole('heading', { name: 'Your workspaces' }),
  ).toBeVisible();
  await expectNavigation(page);
});

test('homepage still redirects a new installation to setup', async ({
  page,
}) => {
  await mockAccount(page);
  await page.route('**/api/v1/auth/me', (route) =>
    route.fulfill({ status: 401, json: { message: 'Sign in required' } }),
  );
  await page.route('**/api/v1/auth/bootstrap', (route) =>
    route.fulfill({
      json: {
        setup_required: true,
        setup_enabled: false,
        mail_available: false,
        oidc_required: false,
        oidc_display_name: null,
      } satisfies Schema['BootstrapInfo'],
    }),
  );
  await page.goto('/');
  await expect(page).toHaveURL(/\/setup$/);
  await expect(
    page.getByRole('heading', { name: 'Set up Convex' }),
  ).toBeVisible();
});

test('system theme changes update the app without reloading', async ({
  page,
}) => {
  await mockAccount(page);
  await page.emulateMedia({ colorScheme: 'light' });
  await page.goto('/');
  await expect(page.locator('html')).toHaveClass('light');
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(page.locator('html')).toHaveClass('dark');
  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('html')).toHaveClass('light');
});

test('sidebar toggles with the keyboard and mobile navigation closes after selecting a workspace', async ({
  page,
}) => {
  await mockAccount(page);
  await page.route('**/api/v1/workspaces/workspace', (route) =>
    route.fulfill({
      json: {
        workspace: {
          id: 'workspace',
          name: 'Product',
          slug: 'product',
          role: 'member',
        },
        members: [],
        teams: [],
      } satisfies Schema['WorkspaceDetail'],
    }),
  );
  await page.goto('/');
  const trigger = page.getByRole('button', { name: 'Toggle navigation' });
  if ((page.viewportSize()?.width ?? 0) < 768) {
    await trigger.click();
    const drawer = page.getByRole('dialog', { name: 'Sidebar' });
    await expect(drawer).toBeVisible();
    await drawer
      .getByRole('button', { name: 'Product workspace menu' })
      .click();
    await page.getByRole('menuitem', { name: 'Settings', exact: true }).click();
    await expect(page).toHaveURL(/\/product$/);
    await expect(drawer).not.toBeVisible();
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  } else {
    const sidebar = page.locator('[data-slot="sidebar"][data-state]');
    await expect(sidebar).toHaveAttribute('data-state', 'expanded');
    await page.keyboard.press('Control+b');
    await expect(sidebar).toHaveAttribute('data-state', 'collapsed');
    await page.keyboard.press('Control+b');
    await expect(sidebar).toHaveAttribute('data-state', 'expanded');
  }
});

test('workspace menu signs out and returns to login', async ({ page }) => {
  const account = await mockAccount(page);
  let signedIn = true;
  await page.route('**/api/v1/auth/me', (route) =>
    signedIn
      ? route.fulfill({ json: account })
      : route.fulfill({ status: 401, json: { message: 'Sign in required' } }),
  );
  await page.route('**/api/v1/auth/logout', (route) => {
    signedIn = false;
    return route.fulfill({ json: { message: 'Signed out.' } });
  });
  await page.goto('/');
  if ((page.viewportSize()?.width ?? 0) < 768)
    await page.getByRole('button', { name: 'Toggle navigation' }).click();
  await page.getByRole('button', { name: 'Product workspace menu' }).click();
  await page.getByRole('menuitem', { name: 'Sign out', exact: true }).click();
  await expect(page).toHaveURL(/\/auth\/login$/);
  await expect(
    page.getByRole('heading', { name: 'Welcome back.' }),
  ).toBeVisible();
});

test('nested workspace switcher remembers selection across account pages and reloads', async ({
  page,
}) => {
  const account = await mockAccount(page);
  const engineering: Schema['Workspace'] = {
    id: 'engineering',
    name: 'Engineering',
    slug: 'engineering-team',
    role: 'member',
  };
  account.workspaces.push(engineering);
  await page.route('**/api/v1/workspaces/engineering', (route) =>
    route.fulfill({
      json: {
        workspace: engineering,
        members: [],
        teams: [],
      } satisfies Schema['WorkspaceDetail'],
    }),
  );
  await page.route('**/api/v1/auth/sessions', (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto('/');
  const mobile = (page.viewportSize()?.width ?? 0) < 768;
  if (mobile)
    await page.getByRole('button', { name: 'Toggle navigation' }).click();
  await page.getByRole('button', { name: 'Product workspace menu' }).click();
  await page.getByRole('menuitem', { name: 'Switch workspace' }).click();
  await expectInside(
    page.locator(
      mobile
        ? '[data-slot="dropdown-menu-content"]'
        : '[data-slot="dropdown-menu-sub-content"]',
    ),
    page.locator('body'),
  );
  await expect(
    page.getByRole('menuitem', { name: /^Product/ }),
  ).toHaveAttribute('aria-current', 'page');
  await expect(
    page.getByRole('menuitem', { name: 'Create a workspace' }),
  ).toHaveCount(0);
  if (mobile) {
    await page.getByRole('menuitem', { name: 'Back', exact: true }).click();
    await expect(
      page.getByRole('menuitem', { name: 'Settings', exact: true }),
    ).toBeVisible();
    await page.getByRole('menuitem', { name: 'Switch workspace' }).click();
  }
  await page
    .getByRole('menuitem', { name: 'Engineering', exact: true })
    .click();
  await expect(page).toHaveURL(/\/engineering-team$/);
  await expect(
    page
      .getByRole('main')
      .getByRole('heading', { name: 'Workspace', exact: true }),
  ).toBeVisible();
  if (mobile) {
    await expect(
      page.getByRole('dialog', { name: 'Sidebar' }),
    ).not.toBeVisible();
    await page.getByRole('button', { name: 'Toggle navigation' }).click();
  }
  await expect(
    page.getByRole('button', { name: 'Engineering workspace menu' }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Your account', exact: true }).click();
  await expect(page).toHaveURL(/\/account$/);
  await page.reload();
  if (mobile)
    await page.getByRole('button', { name: 'Toggle navigation' }).click();
  await page
    .getByRole('button', { name: 'Engineering workspace menu' })
    .click();
  await expect(
    page.getByRole('menuitem', { name: 'View members', exact: true }),
  ).toBeVisible();
});

test('compact role controls stay usable and submit the selected role', async ({
  page,
}) => {
  await mockAccount(page);
  const detail: Schema['WorkspaceDetail'] = {
    workspace: {
      id: 'workspace',
      name: 'Product',
      slug: 'product',
      role: 'owner',
    },
    members: [
      {
        id: 'membership',
        user_id: 'teammate',
        display_name: 'Teammate',
        role: 'member',
        disabled: false,
      },
    ],
    teams: [],
  };
  await page.route('**/api/v1/workspaces/workspace', (route) =>
    route.fulfill({ json: detail }),
  );
  await page.route('**/api/v1/workspaces/workspace/invitations', (route) =>
    route.fulfill({ json: [] }),
  );
  let finishRequest = () => {};
  const pendingRequest = new Promise<void>((resolve) => {
    finishRequest = resolve;
  });
  await page.route('**/api/v1/workspaces/workspace/members', async (route) => {
    await pendingRequest;
    await route.fulfill({ json: { message: 'Role saved.' } });
  });
  await page.goto('/product#people');
  await page.getByRole('button', { name: 'Manage Teammate' }).click();
  const row = page.getByRole('dialog', { name: 'Manage Teammate' });
  const role = row.getByRole('combobox', { name: 'Role for Teammate' });
  await expect(role).toHaveValue('member');
  const selectBox = await role.boundingBox();
  expect(selectBox?.width).toBeGreaterThanOrEqual(100);
  await expectInside(role, row);
  await expectInside(row.getByRole('button', { name: 'Save role' }), row);
  await role.selectOption('admin');
  const request = page.waitForRequest('**/api/v1/workspaces/workspace/members');
  await row.getByRole('button', { name: 'Save role' }).click();
  expect((await request).postDataJSON()).toEqual({
    membership_id: 'membership',
    role: 'admin',
    remove: false,
  });
  await expect(role).toBeDisabled();
  await expect(row.getByRole('button', { name: 'Working…' })).toBeDisabled();
  finishRequest();
  await expect(row.getByRole('status')).toContainText('Role saved');
  await expect(role).toBeEnabled();
  await page.keyboard.press('Escape');
  await expect(
    page.getByRole('button', { name: 'Manage Teammate' }),
  ).toBeFocused();
  await page.getByRole('button', { name: 'Manage Teammate' }).click();
  await page.getByRole('button', { name: 'Close dialog' }).click();
  await expect(
    page.getByRole('button', { name: 'Manage Teammate' }),
  ).toBeFocused();
  await page.screenshot({
    path: `test-results/controls-${test.info().project.name}.png`,
    fullPage: true,
  });
});

test('workspace creation suggests a slug, preserves edits after conflicts, and navigates to the root URL', async ({
  page,
}) => {
  const account = await mockAccount(page);
  account.user.is_instance_admin = true;
  const workspace: Schema['Workspace'] = {
    id: 'new-workspace',
    name: 'Renamed company',
    slug: 'my-company',
    role: 'owner',
  };
  let conflict = true;
  await page.route('**/api/v1/workspaces', async (route) => {
    const body = route.request().postDataJSON();
    expect(body).toEqual({
      name: 'Renamed company',
      slug: 'my-company',
      team_name: 'Engineering',
      team_key: 'ENG',
    });
    if (conflict) {
      conflict = false;
      return route.fulfill({
        status: 409,
        json: {
          code: 'conflict',
          message: 'This workspace URL is already in use. Choose another slug.',
        },
      });
    }
    account.workspaces.push(workspace);
    return route.fulfill({ json: workspace });
  });
  await page.route('**/api/v1/workspaces/new-workspace', (route) =>
    route.fulfill({ json: { workspace, members: [], teams: [] } }),
  );
  await page.route('**/api/v1/workspaces/new-workspace/invitations', (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto('/');
  const name = page.getByLabel('Workspace name', { exact: true });
  const slug = page.getByLabel('Workspace URL', { exact: true });
  await name.fill('Café & Company');
  await expect(slug).toHaveValue('cafe-company');
  await slug.fill('my-company');
  await name.fill('Renamed company');
  await expect(slug).toHaveValue('my-company');
  await slug.fill('system');
  await expect(slug).toHaveAttribute('aria-invalid', 'true');
  await expect(
    page.getByText('This URL is reserved for the application.', {
      exact: false,
    }),
  ).toBeVisible();
  await slug.fill('my-company');
  await page.getByLabel('First team name').fill('Engineering');
  await page.getByLabel('Team key', { exact: true }).fill('ENG');
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme, reducedMotion: 'reduce' });
    await page.evaluate(async () => {
      await document.fonts.ready;
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: `../.impeccable/review/slug-create-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
      animations: 'disabled',
    });
  }
  await page.getByRole('button', { name: 'Create workspace' }).click();
  await expect(page.getByRole('alert')).toContainText('already in use');
  await expect(slug).toHaveValue('my-company');
  await expect(name).toHaveValue('Renamed company');
  await page.getByRole('button', { name: 'Create workspace' }).click();
  await expect(page).toHaveURL(/\/my-company$/);
  await expect(
    page.getByRole('heading', { name: 'Workspace', exact: true }),
  ).toBeVisible();
});

test('initial setup submits the selected slug', async ({ page }) => {
  const account = await mockAccount(page);
  const workspace = account.workspaces[0];
  if (!workspace)
    throw new Error('Expected a workspace in the account fixture');
  workspace.slug = 'our-product';
  let initialized = false;
  await page.route('**/api/v1/auth/bootstrap', (route) =>
    route.fulfill({
      json: {
        setup_required: !initialized,
        setup_enabled: true,
        mail_available: false,
        oidc_required: false,
        oidc_display_name: null,
      },
    }),
  );
  await page.route('**/api/v1/auth/me', (route) =>
    initialized
      ? route.fulfill({ json: account })
      : route.fulfill({ status: 401, json: { message: 'Sign in required' } }),
  );
  await page.route('**/api/v1/auth/setup', (route) => {
    expect(route.request().postDataJSON().workspace_slug).toBe('our-product');
    initialized = true;
    return route.fulfill({ json: account.user });
  });
  await page.route('**/api/v1/workspaces/workspace', (route) =>
    route.fulfill({
      json: { workspace: account.workspaces[0], members: [], teams: [] },
    }),
  );
  await page.route('**/api/v1/workspaces/workspace/invitations', (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto('/');
  await page
    .getByLabel('Installation setup credential')
    .fill('installation-token');
  await page.getByLabel('Your name', { exact: true }).fill('Owner');
  await page.getByLabel('Email', { exact: true }).fill('owner@example.test');
  await page.getByLabel('Password', { exact: true }).fill('a-valid-password');
  await page.getByLabel('Workspace name', { exact: true }).fill('Product');
  await expect(page.getByLabel('Workspace URL', { exact: true })).toHaveValue(
    'product',
  );
  await page.getByLabel('Workspace URL', { exact: true }).fill('our-product');
  await page.getByLabel('First team name').fill('Engineering');
  await page.getByLabel('Team key', { exact: true }).fill('ENG');
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme, reducedMotion: 'reduce' });
    await page.evaluate(async () => {
      await document.fonts.ready;
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: `../.impeccable/review/slug-setup-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
      animations: 'disabled',
    });
  }
  await page.getByRole('button', { name: 'Create workspace' }).click();
  await expect(page).toHaveURL(/\/our-product$/);
  await expect(
    page.getByRole('heading', { name: 'Workspace', exact: true }),
  ).toBeVisible();
});

test('old UUID workspace links redirect to slug URLs and preserve sections', async ({
  page,
}) => {
  const account = await mockAccount(page);
  await page.route('**/api/v1/workspaces/workspace', (route) =>
    route.fulfill({
      json: { workspace: account.workspaces[0], members: [], teams: [] },
    }),
  );
  await page.route('**/api/v1/workspaces/workspace/invitations', (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto('/workspaces/workspace#teams');
  await expect(page).toHaveURL(/\/product#teams$/);
  await expect(
    page.getByRole('heading', { name: 'Teams', exact: true }),
  ).toBeVisible();
});

test('session actions fit within the account page', async ({ page }) => {
  await mockAccount(page);
  await page.route('**/api/v1/auth/sessions', (route) =>
    route.fulfill({
      json: [
        {
          id: 'session',
          current: true,
          created_at: '2026-10-08T12:00:00Z',
          expires_at: '2026-10-15T12:00:00Z',
          last_activity_at: '2026-10-08T12:00:00Z',
        },
      ] satisfies Schema['SessionInfo'][],
    }),
  );
  await page.goto('/account');
  const button = page.getByRole('button', { name: 'Sign out session' });
  await expect(button).toBeVisible();
  await expectInside(button, page.getByRole('main'));
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

async function mockSettings(page: Page, role = 'owner', mail = false) {
  const account = await mockAccount(page);
  const workspace = account.workspaces[0]!;
  workspace.role = role;
  const detail: Schema['WorkspaceDetail'] = {
    workspace,
    members: [
      {
        id: 'owner-membership',
        user_id: 'owner',
        display_name: 'Installation owner',
        role: 'owner',
        disabled: false,
      },
      {
        id: 'teammate',
        user_id: 'teammate',
        display_name: 'Alex Morgan',
        role: 'member',
        disabled: false,
      },
      {
        id: 'disabled',
        user_id: 'disabled',
        display_name: 'Sam Rivera',
        role: 'member',
        disabled: true,
      },
    ],
    teams: [
      {
        id: 'engineering',
        name: 'Engineering',
        key: 'ENG',
        statuses: [
          { id: 'backlog', name: 'Backlog', category: 'backlog', position: 0 },
          { id: 'todo', name: 'Todo', category: 'unstarted', position: 1 },
          {
            id: 'progress',
            name: 'In Progress',
            category: 'started',
            position: 2,
          },
          { id: 'done', name: 'Done', category: 'completed', position: 3 },
          {
            id: 'canceled',
            name: 'Canceled',
            category: 'canceled',
            position: 4,
          },
        ],
      },
    ],
  };
  await page.route('**/api/v1/auth/bootstrap', (route) =>
    route.fulfill({
      json: {
        setup_required: false,
        setup_enabled: false,
        mail_available: mail,
      },
    }),
  );
  await page.route('**/api/v1/workspaces/workspace', (route) =>
    route.fulfill({ json: detail }),
  );
  await page.route('**/api/v1/workspaces/workspace/invitations', (route) =>
    route.fulfill({ json: [] }),
  );
  return detail;
}

test('settings screens use one sidebar and fit desktop and mobile in both themes', async ({
  page,
}) => {
  await mockSettings(page, 'owner', true);
  const mobile = test.info().project.name === 'mobile';
  await page.setViewportSize(
    mobile ? { width: 390, height: 844 } : { width: 1613, height: 985 },
  );
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme, reducedMotion: 'reduce' });
    for (const [screen, hash, heading] of [
      ['workspace', '', 'Workspace'],
      ['teams', '#teams', 'Teams'],
      ['members', '#people', 'Members'],
      ['create-team', '#create-team', 'Create a new team'],
    ]) {
      await page.goto(`/product${hash}`);
      await expect(
        page.getByRole('heading', { name: heading, exact: true }),
      ).toBeVisible();
      await page.evaluate(async () => {
        await document.fonts.ready;
        window.scrollTo(0, 0);
      });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      ).toBe(true);
      await page.screenshot({
        path: `../.impeccable/review/settings-${screen}-${test.info().project.name}-${colorScheme}.png`,
        fullPage: true,
        animations: 'disabled',
      });
    }
    await page.goto('/product#people');
    await page.getByRole('button', { name: 'Invite', exact: true }).click();
    await expect(
      page.getByRole('dialog', { name: 'Invite to your workspace' }),
    ).toBeVisible();
    await expect(page.getByLabel('Email', { exact: true })).toBeFocused();
    await page.screenshot({
      path: `../.impeccable/review/settings-invite-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
      animations: 'disabled',
    });
    await page.keyboard.press('Escape');
    await expect(
      page.getByRole('button', { name: 'Invite', exact: true }),
    ).toBeFocused();
  }
  if (mobile)
    await page.getByRole('button', { name: 'Toggle navigation' }).click();
  const navigation = page.getByRole('navigation', {
    name: 'Settings navigation',
  });
  await expect(navigation).toBeVisible();
  await expect(
    page.getByRole('navigation', { name: 'Main navigation' }),
  ).toHaveCount(0);
  await expect(
    navigation.getByRole('link', { name: 'Members' }),
  ).toHaveAttribute('aria-current', 'page');
  await page.getByLabel('Search settings…').fill('teams');
  await expect(
    navigation.getByRole('link', { name: 'Workspace', exact: true }),
  ).toHaveCount(0);
  await navigation.getByRole('link', { name: 'Teams', exact: true }).click();
  if (mobile) {
    await expect(
      page.getByRole('dialog', { name: 'Sidebar' }),
    ).not.toBeVisible();
    await page.getByRole('button', { name: 'Toggle navigation' }).click();
  }
  await page.getByRole('link', { name: 'Back to app' }).click();
  await expect(
    page.getByRole('heading', { name: 'Your workspaces' }),
  ).toBeVisible();
  await expectNavigation(page);
});

test('team creation preserves drafts on conflict and supports rename, search and CSV export', async ({
  page,
}) => {
  const detail = await mockSettings(page);
  let conflict = true;
  await page.route('**/api/v1/workspaces/workspace/teams', (route) => {
    expect(route.request().postDataJSON()).toEqual({
      name: 'Design',
      key: 'UX',
    });
    if (conflict) {
      conflict = false;
      return route.fulfill({
        status: 409,
        json: { message: 'Team identifier already exists.' },
      });
    }
    detail.teams.push({
      id: 'design',
      name: 'Design',
      key: 'UX',
      statuses: detail.teams[0]!.statuses,
    });
    return route.fulfill({ json: { message: 'Team created.' } });
  });
  await page.route(
    '**/api/v1/workspaces/workspace/teams/design/rename',
    (route) => {
      detail.teams[1]!.name = route.request().postDataJSON().name;
      return route.fulfill({ json: { message: 'Team renamed.' } });
    },
  );
  await page.goto('/product#create-team');
  await page.getByLabel('Team name', { exact: true }).fill('Design');
  await expect(page.getByLabel('Team key', { exact: true })).toHaveValue('DES');
  await page.getByLabel('Team key', { exact: true }).fill('UX');
  await page.getByRole('button', { name: 'Create team', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('already exists');
  await expect(page.getByLabel('Team name', { exact: true })).toHaveValue(
    'Design',
  );
  await expect(page.getByLabel('Team key', { exact: true })).toHaveValue('UX');
  await page.getByRole('button', { name: 'Create team', exact: true }).click();
  await expect(page).toHaveURL(/#teams$/);
  await page.getByRole('button', { name: 'Design', exact: true }).click();
  await expect(
    page.getByRole('list', { name: 'Design workflow' }),
  ).toContainText('Todo');
  await page.getByLabel('Name for UX').fill('Product Design');
  await page.getByRole('button', { name: 'Save team name' }).click();
  await expect(page.getByRole('status')).toContainText('Team renamed');
  await page.keyboard.press('Escape');
  await expect(
    page.getByRole('button', { name: 'Product Design', exact: true }),
  ).toBeFocused();
  await page.getByLabel('Filter by name or identifier…').fill('UX');
  await expect(
    page.getByRole('button', { name: 'Engineering', exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole('button', { name: 'Product Design', exact: true }),
  ).toBeVisible();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export CSV' }).click();
  expect((await download).suggestedFilename()).toBe('product-teams.csv');
});

test('batch invitations retain successful links and only retry remaining recipients', async ({
  page,
}) => {
  await mockSettings(page, 'owner', true);
  const recipients: string[] = [];
  let fail = true;
  await page.route('**/api/v1/workspaces/workspace/invitations', (route) => {
    if (route.request().method() === 'GET') return route.fulfill({ json: [] });
    const body = route.request().postDataJSON();
    expect(body.role).toBe('admin');
    recipients.push(body.deliver_to);
    if (body.deliver_to === 'second@example.test' && fail) {
      fail = false;
      return route.fulfill({
        status: 503,
        json: { message: 'Try again later.' },
      });
    }
    return route.fulfill({
      json: {
        id: body.deliver_to,
        email_delivered: false,
        expires_at: '2026-10-16T12:00:00Z',
        url: `http://127.0.0.1:5175/invite#${body.deliver_to}`,
      },
    });
  });
  await page.goto('/product#people');
  await page.getByRole('button', { name: 'Invite', exact: true }).click();
  await page.getByLabel('Email', { exact: true }).fill('not-an-email');
  await page.getByRole('button', { name: 'Send invites' }).click();
  await expect(page.getByRole('alert')).toContainText('Check the email');
  expect(recipients).toHaveLength(0);
  await page.getByLabel('Invitation role').selectOption('admin');
  await page
    .getByLabel('Email', { exact: true })
    .fill('first@example.test, first@example.test\nsecond@example.test');
  await page.getByRole('button', { name: 'Send invites' }).click();
  await expect(page.getByRole('alert')).toContainText('Try again later');
  await expect(page.getByLabel('Email', { exact: true })).toHaveValue(
    'second@example.test',
  );
  await expect(
    page.getByLabel('Invitation link for first@example.test'),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Send invites' }).click();
  await expect(
    page.getByLabel('Invitation link for second@example.test'),
  ).toBeVisible();
  expect(recipients).toEqual([
    'first@example.test',
    'second@example.test',
    'second@example.test',
  ]);
});

test('members can search and filter but cannot create teams or change workspace access', async ({
  page,
}) => {
  await mockSettings(page, 'member');
  await page.goto('/product');
  await expect(
    page.getByLabel('Workspace name', { exact: true }),
  ).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Save changes' })).toHaveCount(
    0,
  );
  await page.goto('/product#teams');
  await expect(page.getByRole('link', { name: 'Create team' })).toHaveCount(0);
  await page.goto('/product#create-team');
  await expect(
    page.getByRole('heading', { name: 'Workspace', exact: true }),
  ).toBeVisible();
  await page.goto('/product#people');
  await expect(
    page.getByRole('button', { name: 'Invite', exact: true }),
  ).toHaveCount(0);
  await expect(page.getByRole('button', { name: /^Manage / })).toHaveCount(0);
  await page.getByLabel('Search by name…').fill('no such person');
  await expect(page.getByText('No members match your filters.')).toBeVisible();
  await page.getByLabel('Search by name…').fill('');
  await page.getByLabel('Filter members').selectOption('disabled');
  await expect(page.getByText('Sam Rivera', { exact: true })).toBeVisible();
  await expect(page.getByText('Alex Morgan', { exact: true })).toHaveCount(0);
});

test('member removal restores focus to the directory when its row disappears', async ({
  page,
}) => {
  const detail = await mockSettings(page);
  await page.route('**/api/v1/workspaces/workspace/members', (route) => {
    expect(route.request().postDataJSON()).toEqual({
      membership_id: 'teammate',
      role: null,
      remove: true,
    });
    detail.members = detail.members.filter(
      (member) => member.id !== 'teammate',
    );
    return route.fulfill({ json: { message: 'Membership removed.' } });
  });
  await page.goto('/product#people');
  await page.getByRole('button', { name: 'Manage Alex Morgan' }).click();
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByText('Alex Morgan', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  await page
    .getByRole('button', { name: 'Confirm removal', exact: true })
    .click();
  await expect(
    page.getByRole('dialog', { name: 'Manage Alex Morgan' }),
  ).not.toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Name', exact: true }),
  ).toBeFocused();
  await expect(page.getByText('Alex Morgan', { exact: true })).toHaveCount(0);
});
