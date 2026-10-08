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
    workspaces: [{ id: "workspace", name: "Product", role: "owner" }],
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

test("authenticated homepage renders inside the app shell", async ({
  page,
}) => {
  await mockAccount(page);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Your workspaces" }),
  ).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Main navigation" }),
  ).toBeVisible();
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
  await expect(
    page.getByRole("navigation", { name: "Main navigation" }),
  ).toBeVisible();
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
    page.getByRole("heading", { name: "Make room for your team." }),
  ).toBeVisible();
});

test("compact role controls stay usable and submit the selected role", async ({
  page,
}) => {
  await mockAccount(page);
  const detail: Schema["WorkspaceDetail"] = {
    workspace: { id: "workspace", name: "Product", role: "owner" },
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
  await page.goto("/workspaces/workspace");
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
