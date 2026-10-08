import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  workers: 1,
  use: { baseURL: "http://127.0.0.1:5174", trace: "retain-on-failure" },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"] } },
    {
      name: "mobile",
      use: { ...devices["iPhone 13"], defaultBrowserType: "chromium" },
    },
  ],
  webServer: [
    {
      command: "mise run dev:e2e:api",
      cwd: "..",
      url: "http://127.0.0.1:3001/api/v1/health",
      timeout: 120_000,
      reuseExistingServer: false,
    },
    {
      command: "pnpm dev --port 5174",
      env: { CONVEX_API_PROXY: "http://127.0.0.1:3001" },
      url: "http://127.0.0.1:5174",
      reuseExistingServer: false,
    },
  ],
});
