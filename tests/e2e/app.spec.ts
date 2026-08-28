import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
  await page.reload();
});

test('imports responses and completes teacher-reviewed feedback', async ({ page }) => {
  await expect(page).toHaveTitle(/Rubric Comment Queue/);
  await expect(page.locator('h1')).toHaveCount(1);
  await page.getByRole('button', { name: 'Add the first responses' }).click();
  await page.getByLabel('Response text').fill('# Roster 12\nA clear claim with one example.\n\n---\n\n# Roster 13\nA second draft to review.');
  await page.getByRole('button', { name: 'Add to queue' }).click();
  await expect(page.getByText('0 / 2 ready')).toBeVisible();

  await page.getByRole('button', { name: /Connect evidence/ }).click();
  const feedback = page.getByLabel(/Feedback draft/);
  await expect(feedback).toHaveValue(/Your evidence is relevant/);
  await feedback.fill('Your example supports the claim. Explain why this detail matters to your reader.');
  await page.getByLabel(/One personal next step/).fill('Add one sentence connecting the example to your claim.');
  await page.keyboard.press('Control+Enter');
  await expect(page.getByText('1 / 2 ready')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Roster 13' })).toBeVisible();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export all CSV' }).click();
  expect((await download).suggestedFilename()).toMatch(/rubric-feedback.*\.csv/);
});

test('has no serious accessibility findings in empty and legal states', async ({ page }) => {
  let results = await new AxeBuilder({ page }).analyze();
  expect(results.violations.filter((issue) => ['serious', 'critical'].includes(issue.impact ?? ''))).toEqual([]);
  await page.goto('/privacy');
  await expect(page.getByRole('heading', { name: 'Privacy, in plain language' })).toBeVisible();
  results = await new AxeBuilder({ page }).analyze();
  expect(results.violations.filter((issue) => ['serious', 'critical'].includes(issue.impact ?? ''))).toEqual([]);
});

test('opens the paid unlock without gating the free workflow', async ({ page }) => {
  await page.getByRole('button', { name: 'Desk Pass' }).click();
  await expect(page.getByText('$29')).toBeVisible();
  await expect(page.getByRole('link', { name: /Buy Desk Pass/ })).toHaveAttribute('href', 'https://api.sociobot.in/api/v1/products/rubric-comment-queue/checkout');
  await expect(page.getByLabel('Paste license token')).toBeVisible();
});

test('keeps the local workspace available offline', async ({ page, context }) => {
  await context.setOffline(true);
  await expect(page.getByText('Offline · local save on')).toBeVisible();
  await page.getByRole('button', { name: 'Add the first responses' }).click();
  await page.getByLabel('Response text').fill('# Offline draft\nThis remains on this device.');
  await page.getByRole('button', { name: 'Add to queue' }).click();
  await expect(page.getByRole('heading', { name: 'Offline draft' })).toBeVisible();
  await context.setOffline(false);
});
