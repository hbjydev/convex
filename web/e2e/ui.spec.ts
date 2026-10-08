import { expect, type Locator, type Page, test } from "@playwright/test";
import type { components } from "../src/api/schema";

type Schema = components["schemas"];

async function mockAccount(page: Page) {
  const account: Schema["AccountResponse"] = {
    user: {
      id: "owner",
      display_name: "Installation owner",
      is_instance_admin: false,
    },
    workspaces: [
      { id: "workspace", name: "Product", slug: "product", role: "owner" },
    ],
  };
  await page.route("**/api/v1/auth/bootstrap", (route) =>
    route.fulfill({
      json: {
        setup_required: false,
        setup_enabled: false,
        mail_available: false,
      } satisfies Schema["BootstrapInfo"],
    }),
  );
  await page.route("**/api/v1/auth/me", (route) =>
    route.fulfill({ json: account }),
  );
  return account;
}

async function expectInside(control: Locator, container: Locator) {
  const box = await control.boundingBox();
  const boundary = await container.boundingBox();
  expect(box).not.toBeNull();
  expect(boundary).not.toBeNull();
  if (!box || !boundary) throw new Error("Control or container is not visible");
  expect(box.x).toBeGreaterThanOrEqual(boundary.x);
  expect(box.x + box.width).toBeLessThanOrEqual(
    boundary.x + boundary.width + 1,
  );
}

async function expectNavigation(page: Page) {
  const navigation = page.getByRole("navigation", { name: "Main navigation" });
  if ((page.viewportSize()?.width ?? 0) < 768) {
    await page.getByRole("button", { name: "Toggle navigation" }).click();
    await expect(navigation).toBeVisible();
    await page.getByRole("button", { name: "Close navigation" }).click();
  } else {
    await expect(navigation).toBeVisible();
  }
}

test("authenticated homepage renders inside the app shell", async ({
  page,
}) => {
  await mockAccount(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Your workspaces" }),
  ).toBeVisible();
  await expectNavigation(page);
  await expect(
    page.getByRole("main").getByRole("heading", { name: "Your workspaces" }),
  ).toBeVisible();
});

test("homepage redirects to sign in and keeps the shell after login", async ({
  page,
}) => {
  const account = await mockAccount(page);
  let signedIn = false;
  await page.route("**/api/v1/auth/me", (route) =>
    signedIn
      ? route.fulfill({ json: account })
      : route.fulfill({ status: 401, json: { message: "Sign in required" } }),
  );
  await page.route("**/api/v1/auth/login", (route) => {
    signedIn = true;
    return route.fulfill({ json: account });
  });
  await page.goto("/");
  await expect(page).toHaveURL(/\/auth\/login$/);
  await expect(
    page.getByRole("textbox", { name: "Email", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("textbox", { name: "Email", exact: true })
    .fill("owner@example.test");
  await page.getByLabel("Password", { exact: true }).fill("a-valid-password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(
    page.getByRole("main").getByRole("heading", { name: "Your workspaces" }),
  ).toBeVisible();
  await expectNavigation(page);
});

test("homepage still redirects a new installation to setup", async ({
  page,
}) => {
  await mockAccount(page);
  await page.route("**/api/v1/auth/me", (route) =>
    route.fulfill({ status: 401, json: { message: "Sign in required" } }),
  );
  await page.route("**/api/v1/auth/bootstrap", (route) =>
    route.fulfill({
      json: {
        setup_required: true,
        setup_enabled: false,
        mail_available: false,
      } satisfies Schema["BootstrapInfo"],
    }),
  );
  await page.goto("/");
  await expect(page).toHaveURL(/\/setup$/);
  await expect(
    page.getByRole("heading", { name: "Set up Convex" }),
  ).toBeVisible();
});

test("system theme changes update the app without reloading", async ({
  page,
}) => {
  await mockAccount(page);
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  await expect(page.locator("html")).toHaveClass("light");
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveClass("dark");
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page.locator("html")).toHaveClass("light");
});

test("sidebar toggles with the keyboard and mobile navigation closes after selecting a workspace", async ({
  page,
}) => {
  await mockAccount(page);
  await page.route("**/api/v1/workspaces/workspace", (route) =>
    route.fulfill({
      json: {
        workspace: {
          id: "workspace",
          name: "Product",
          slug: "product",
          role: "member",
        },
        members: [],
        teams: [],
      } satisfies Schema["WorkspaceDetail"],
    }),
  );
  await page.goto("/");
  const trigger = page.getByRole("button", { name: "Toggle navigation" });
  if ((page.viewportSize()?.width ?? 0) < 768) {
    await trigger.click();
    const drawer = page.getByRole("dialog", { name: "Sidebar" });
    await expect(drawer).toBeVisible();
    await drawer.getByRole("link", { name: "People", exact: true }).click();
    await expect(page).toHaveURL(/\/product#people$/);
    await expect(drawer).not.toBeVisible();
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
  } else {
    const sidebar = page.locator('[data-slot="sidebar"][data-state]');
    await expect(sidebar).toHaveAttribute("data-state", "expanded");
    await page.keyboard.press("Control+b");
    await expect(sidebar).toHaveAttribute("data-state", "collapsed");
    await page.keyboard.press("Control+b");
    await expect(sidebar).toHaveAttribute("data-state", "expanded");
  }
});

test("workspace menu signs out and returns to login", async ({ page }) => {
  const account = await mockAccount(page);
  let signedIn = true;
  await page.route("**/api/v1/auth/me", (route) =>
    signedIn
      ? route.fulfill({ json: account })
      : route.fulfill({ status: 401, json: { message: "Sign in required" } }),
  );
  await page.route("**/api/v1/auth/logout", (route) => {
    signedIn = false;
    return route.fulfill({ json: { message: "Signed out." } });
  });
  await page.goto("/");
  if ((page.viewportSize()?.width ?? 0) < 768)
    await page.getByRole("button", { name: "Toggle navigation" }).click();
  await page.getByRole("button", { name: "Product workspace menu" }).click();
  await page.getByRole("menuitem", { name: "Sign out", exact: true }).click();
  await expect(page).toHaveURL(/\/auth\/login$/);
  await expect(
    page.getByRole("heading", { name: "Welcome back." }),
  ).toBeVisible();
});

test("nested workspace switcher remembers selection across account pages and reloads", async ({
  page,
}) => {
  const account = await mockAccount(page);
  const engineering: Schema["Workspace"] = {
    id: "engineering",
    name: "Engineering",
    slug: "engineering-team",
    role: "member",
  };
  account.workspaces.push(engineering);
  await page.route("**/api/v1/workspaces/engineering", (route) =>
    route.fulfill({
      json: {
        workspace: engineering,
        members: [],
        teams: [],
      } satisfies Schema["WorkspaceDetail"],
    }),
  );
  await page.route("**/api/v1/auth/sessions", (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto("/");
  const mobile = (page.viewportSize()?.width ?? 0) < 768;
  if (mobile)
    await page.getByRole("button", { name: "Toggle navigation" }).click();
  await page.getByRole("button", { name: "Product workspace menu" }).click();
  await page.getByRole("menuitem", { name: "Switch workspace" }).click();
  await expectInside(
    page.locator(
      mobile
        ? '[data-slot="dropdown-menu-content"]'
        : '[data-slot="dropdown-menu-sub-content"]',
    ),
    page.locator("body"),
  );
  await expect(
    page.getByRole("menuitem", { name: /^Product/ }),
  ).toHaveAttribute("aria-current", "page");
  await expect(
    page.getByRole("menuitem", { name: "Create a workspace" }),
  ).toHaveCount(0);
  if (mobile) {
    await page.getByRole("menuitem", { name: "Back", exact: true }).click();
    await expect(
      page.getByRole("menuitem", { name: "Settings", exact: true }),
    ).toBeVisible();
    await page.getByRole("menuitem", { name: "Switch workspace" }).click();
  }
  await page
    .getByRole("menuitem", { name: "Engineering", exact: true })
    .click();
  await expect(page).toHaveURL(/\/engineering-team$/);
  await expect(
    page
      .getByRole("main")
      .getByRole("heading", { name: "Engineering", exact: true }),
  ).toBeVisible();
  if (mobile) {
    await expect(
      page.getByRole("dialog", { name: "Sidebar" }),
    ).not.toBeVisible();
    await page.getByRole("button", { name: "Toggle navigation" }).click();
  }
  await expect(
    page.getByRole("button", { name: "Engineering workspace menu" }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Your account", exact: true }).click();
  await expect(page).toHaveURL(/\/account$/);
  await page.reload();
  if (mobile)
    await page.getByRole("button", { name: "Toggle navigation" }).click();
  await page
    .getByRole("button", { name: "Engineering workspace menu" })
    .click();
  await expect(
    page.getByRole("menuitem", { name: "View members", exact: true }),
  ).toBeVisible();
});

test("compact role controls stay usable and submit the selected role", async ({
  page,
}) => {
  await mockAccount(page);
  const detail: Schema["WorkspaceDetail"] = {
    workspace: {
      id: "workspace",
      name: "Product",
      slug: "product",
      role: "owner",
    },
    members: [
      {
        id: "membership",
        user_id: "teammate",
        display_name: "Teammate",
        role: "member",
        disabled: false,
      },
    ],
    teams: [],
  };
  await page.route("**/api/v1/workspaces/workspace", (route) =>
    route.fulfill({ json: detail }),
  );
  await page.route("**/api/v1/workspaces/workspace/invitations", (route) =>
    route.fulfill({ json: [] }),
  );
  let finishRequest = () => {};
  const pendingRequest = new Promise<void>((resolve) => {
    finishRequest = resolve;
  });
  await page.route("**/api/v1/workspaces/workspace/members", async (route) => {
    await pendingRequest;
    await route.fulfill({ json: { message: "Role saved." } });
  });
  await page.goto("/product");
  const row = page.getByRole("listitem").filter({ hasText: "Teammate" });
  const role = row.getByRole("combobox", { name: "Role for Teammate" });
  await expect(role).toHaveValue("member");
  const selectBox = await role.boundingBox();
  const removeBox = await row
    .getByRole("button", { name: "Remove", exact: true })
    .boundingBox();
  expect(selectBox?.width).toBeGreaterThanOrEqual(100);
  if (!selectBox || !removeBox)
    throw new Error("Member controls are not visible");
  expect(removeBox.x).toBeGreaterThanOrEqual(selectBox.x + selectBox.width);
  await expectInside(role, row);
  await expectInside(row.getByRole("button", { name: "Save role" }), row);
  await role.selectOption("admin");
  const request = page.waitForRequest("**/api/v1/workspaces/workspace/members");
  await row.getByRole("button", { name: "Save role" }).click();
  expect((await request).postDataJSON()).toEqual({
    membership_id: "membership",
    role: "admin",
    remove: false,
  });
  await expect(role).toBeDisabled();
  await expect(row.getByRole("button", { name: "Working…" })).toBeDisabled();
  finishRequest();
  await expect(row.getByRole("status")).toContainText("Role saved");
  await expect(role).toBeEnabled();
  await page.screenshot({
    path: `test-results/controls-${test.info().project.name}.png`,
    fullPage: true,
  });
});

test("workspace creation suggests a slug, preserves edits after conflicts, and navigates to the root URL", async ({
  page,
}) => {
  const account = await mockAccount(page);
  account.user.is_instance_admin = true;
  const workspace: Schema["Workspace"] = {
    id: "new-workspace",
    name: "Renamed company",
    slug: "my-company",
    role: "owner",
  };
  let conflict = true;
  await page.route("**/api/v1/workspaces", async (route) => {
    const body = route.request().postDataJSON();
    expect(body).toEqual({
      name: "Renamed company",
      slug: "my-company",
      team_name: "Engineering",
      team_key: "ENG",
    });
    if (conflict) {
      conflict = false;
      return route.fulfill({
        status: 409,
        json: {
          code: "conflict",
          message: "This workspace URL is already in use. Choose another slug.",
        },
      });
    }
    account.workspaces.push(workspace);
    return route.fulfill({ json: workspace });
  });
  await page.route("**/api/v1/workspaces/new-workspace", (route) =>
    route.fulfill({ json: { workspace, members: [], teams: [] } }),
  );
  await page.route("**/api/v1/workspaces/new-workspace/invitations", (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto("/");
  const name = page.getByLabel("Workspace name", { exact: true });
  const slug = page.getByLabel("Workspace URL", { exact: true });
  await name.fill("Café & Company");
  await expect(slug).toHaveValue("cafe-company");
  await slug.fill("my-company");
  await name.fill("Renamed company");
  await expect(slug).toHaveValue("my-company");
  await slug.fill("system");
  await expect(slug).toHaveAttribute("aria-invalid", "true");
  await expect(
    page.getByText("This URL is reserved for the application.", {
      exact: false,
    }),
  ).toBeVisible();
  await slug.fill("my-company");
  await page.getByLabel("First team name").fill("Engineering");
  await page.getByLabel("Team key", { exact: true }).fill("ENG");
  for (const colorScheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme, reducedMotion: "reduce" });
    await page.evaluate(async () => {
      await document.fonts.ready;
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: `../.impeccable/review/slug-create-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
      animations: "disabled",
    });
  }
  await page.getByRole("button", { name: "Create workspace" }).click();
  await expect(page.getByRole("alert")).toContainText("already in use");
  await expect(slug).toHaveValue("my-company");
  await expect(name).toHaveValue("Renamed company");
  await page.getByRole("button", { name: "Create workspace" }).click();
  await expect(page).toHaveURL(/\/my-company$/);
  await expect(
    page.getByRole("heading", { name: "Renamed company", exact: true }),
  ).toBeVisible();
});

test("initial setup submits the selected slug", async ({ page }) => {
  const account = await mockAccount(page);
  const workspace = account.workspaces[0];
  if (!workspace)
    throw new Error("Expected a workspace in the account fixture");
  workspace.slug = "our-product";
  let initialized = false;
  await page.route("**/api/v1/auth/bootstrap", (route) =>
    route.fulfill({
      json: {
        setup_required: !initialized,
        setup_enabled: true,
        mail_available: false,
      },
    }),
  );
  await page.route("**/api/v1/auth/me", (route) =>
    initialized
      ? route.fulfill({ json: account })
      : route.fulfill({ status: 401, json: { message: "Sign in required" } }),
  );
  await page.route("**/api/v1/auth/setup", (route) => {
    expect(route.request().postDataJSON().workspace_slug).toBe("our-product");
    initialized = true;
    return route.fulfill({ json: account.user });
  });
  await page.route("**/api/v1/workspaces/workspace", (route) =>
    route.fulfill({
      json: { workspace: account.workspaces[0], members: [], teams: [] },
    }),
  );
  await page.route("**/api/v1/workspaces/workspace/invitations", (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto("/");
  await page
    .getByLabel("Installation setup credential")
    .fill("installation-token");
  await page.getByLabel("Your name", { exact: true }).fill("Owner");
  await page.getByLabel("Email", { exact: true }).fill("owner@example.test");
  await page.getByLabel("Password", { exact: true }).fill("a-valid-password");
  await page.getByLabel("Workspace name", { exact: true }).fill("Product");
  await expect(page.getByLabel("Workspace URL", { exact: true })).toHaveValue(
    "product",
  );
  await page.getByLabel("Workspace URL", { exact: true }).fill("our-product");
  await page.getByLabel("First team name").fill("Engineering");
  await page.getByLabel("Team key", { exact: true }).fill("ENG");
  for (const colorScheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme, reducedMotion: "reduce" });
    await page.evaluate(async () => {
      await document.fonts.ready;
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: `../.impeccable/review/slug-setup-${test.info().project.name}-${colorScheme}.png`,
      fullPage: true,
      animations: "disabled",
    });
  }
  await page.getByRole("button", { name: "Create workspace" }).click();
  await expect(page).toHaveURL(/\/our-product$/);
  await expect(
    page.getByRole("heading", { name: "Product", exact: true }),
  ).toBeVisible();
});

test("old UUID workspace links redirect to slug URLs and preserve sections", async ({
  page,
}) => {
  const account = await mockAccount(page);
  await page.route("**/api/v1/workspaces/workspace", (route) =>
    route.fulfill({
      json: { workspace: account.workspaces[0], members: [], teams: [] },
    }),
  );
  await page.route("**/api/v1/workspaces/workspace/invitations", (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto("/workspaces/workspace#teams");
  await expect(page).toHaveURL(/\/product#teams$/);
  await expect(
    page.getByRole("heading", { name: "Product", exact: true }),
  ).toBeVisible();
});

test("session actions fit within the account page", async ({ page }) => {
  await mockAccount(page);
  await page.route("**/api/v1/auth/sessions", (route) =>
    route.fulfill({
      json: [
        {
          id: "session",
          current: true,
          created_at: "2026-10-08T12:00:00Z",
          expires_at: "2026-10-15T12:00:00Z",
          last_activity_at: "2026-10-08T12:00:00Z",
        },
      ] satisfies Schema["SessionInfo"][],
    }),
  );
  await page.goto("/account");
  const button = page.getByRole("button", { name: "Sign out session" });
  await expect(button).toBeVisible();
  await expectInside(button, page.getByRole("main"));
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});
