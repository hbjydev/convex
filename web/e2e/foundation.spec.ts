import { expect, test } from '@playwright/test';

test('the frontend makes a real documented request and can retry', async ({
  page,
}) => {
  await page.goto('/system');
  await expect(page.getByRole('status')).toContainText('Ready to build');
  await expect(page.getByText('0.1.0', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Check again' }).click();
  await expect(page.getByRole('status')).toContainText('Ready to build');
  await expect(
    page.getByRole('link', { name: 'Open specification' }),
  ).toHaveAttribute('href', '/api/openapi.json');
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: `test-results/foundation-${test.info().project.name}.png`,
    fullPage: true,
  });
});

test('a network failure offers recovery', async ({ page }) => {
  await page.route('**/api/v1/health', (route) => route.abort());
  await page.goto('/system');
  await expect(page.getByRole('status')).toContainText(
    'Connection unavailable',
  );
  await page.unroute('**/api/v1/health');
  await page.getByRole('button', { name: 'Check again' }).click();
  await expect(page.getByRole('status')).toContainText('Ready to build');
});
