import { defineConfig, devices } from '@playwright/test';
import { apiOrigin, apiPort, webOrigin, webPort } from './e2e/environment';

export default defineConfig({
  testDir: './e2e',
  workers: 1,
  use: { baseURL: webOrigin, trace: 'retain-on-failure' },
  projects: [
    { name: 'desktop', use: { ...devices['Desktop Chrome'] } },
    {
      name: 'mobile',
      use: { ...devices['iPhone 13'], defaultBrowserType: 'chromium' },
    },
  ],
  webServer: [
    {
      command:
        process.env.CONVEX_E2E_OIDC === '1'
          ? 'mise run dev:e2e:oidc:api'
          : 'mise run dev:e2e:api',
      cwd: '..',
      env: {
        CONVEX_E2E_API_PORT: String(apiPort),
        CONVEX_E2E_WEB_PORT: String(webPort),
      },
      url: `${apiOrigin}/api/v1/health`,
      timeout: 120_000,
      reuseExistingServer: false,
    },
    {
      command: `pnpm dev --port ${webPort} --strictPort`,
      env: { CONVEX_API_PROXY: apiOrigin },
      url: webOrigin,
      reuseExistingServer: false,
    },
  ],
});
