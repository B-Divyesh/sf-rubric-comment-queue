import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

async function importOneResponse(page: import('@playwright/test').Page) {
  await page.getByRole('button', { name: 'Add the first responses' }).click();
  await page.getByLabel('Response text').fill('# Roster 12\nA clear claim with one example.');
  await page.getByRole('button', { name: 'Add to queue' }).click();
  await expect(page.getByRole('heading', { name: 'Roster 12' })).toBeVisible();
}

async function expectNoSeriousAxeFindings(page: import('@playwright/test').Page) {
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations.filter((issue) => ['serious', 'critical'].includes(issue.impact ?? ''))).toEqual([]);
}

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
  await expectNoSeriousAxeFindings(page);
  await page.goto('/privacy');
  await expect(page.getByRole('heading', { name: 'Privacy, in plain language' })).toBeVisible();
  await expectNoSeriousAxeFindings(page);
});

test('keeps every dark-theme state readable', async ({ page }) => {
  await page.getByRole('button', { name: 'Use dark theme' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expectNoSeriousAxeFindings(page);

  await importOneResponse(page);
  await expectNoSeriousAxeFindings(page);
  await page.getByRole('button', { name: 'Desk Pass' }).click();
  await expectNoSeriousAxeFindings(page);

  await page.goto('/privacy');
  await expectNoSeriousAxeFindings(page);
  await page.goto('/terms');
  await expectNoSeriousAxeFindings(page);
});

test('visible dialog close and cancel controls close dialogs and restore focus', async ({ page }) => {
  const importOpener = page.getByRole('button', { name: 'Add the first responses' });
  const importDialog = page.locator('dialog[aria-labelledby="import-title"]');
  await importOpener.focus();
  await page.keyboard.press('Enter');
  await expect(importDialog).toHaveJSProperty('open', true);
  await page.getByRole('button', { name: 'Close import dialog' }).focus();
  await page.keyboard.press('Enter');
  await expect(importDialog).toHaveJSProperty('open', false);
  await expect(importOpener).toBeFocused();

  await importOpener.click();
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(importDialog).toHaveJSProperty('open', false);
  await expect(importOpener).toBeFocused();

  await importOneResponse(page);
  const commentOpener = page.getByRole('button', { name: 'Add a comment block' });
  const commentDialog = page.locator('dialog[aria-labelledby="comment-title"]');
  await commentOpener.focus();
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: 'Close comment dialog' }).focus();
  await page.keyboard.press('Enter');
  await expect(commentDialog).toHaveJSProperty('open', false);
  await expect(commentOpener).toBeFocused();

  await commentOpener.click();
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(commentDialog).toHaveJSProperty('open', false);
  await expect(commentOpener).toBeFocused();

  const backupOpener = page.getByRole('button', { name: 'Desk Pass' });
  const backupDialog = page.locator('dialog[aria-labelledby="backup-title"]');
  await backupOpener.focus();
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: 'Close backup dialog' }).focus();
  await page.keyboard.press('Enter');
  await expect(backupDialog).toHaveJSProperty('open', false);
  await expect(backupOpener).toBeFocused();
});

test('recovers from a malformed cached license verdict without blocking startup', async ({ page }) => {
  const pageErrors: Error[] = [];
  page.on('pageerror', (error) => pageErrors.push(error));
  await page.evaluate(() => localStorage.setItem('sb_license_verdict:rubric-comment-queue', '{not valid json'));
  await page.reload();
  await expect(page.locator('main')).toHaveAttribute('aria-busy', 'false');
  await expect(page.getByRole('button', { name: 'Add the first responses' })).toBeVisible();
  await expect.poll(() => page.evaluate(() => localStorage.getItem('sb_license_verdict:rubric-comment-queue'))).toBeNull();
  expect(pageErrors).toEqual([]);
});

test('keeps persistent controls at least 44px at 390px', async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== 'mobile', 'Hit-area contract is measured at the mobile breakpoint.');
  await page.setViewportSize({ width: 390, height: 844 });
  await importOneResponse(page);
  const measurements = await page.locator('.brand, .topbar .small, .queue .text-button, footer a[href="/privacy"], footer a[href="/terms"]').evaluateAll((elements) =>
    elements.map((element) => {
      const { width, height } = element.getBoundingClientRect();
      return { label: element.textContent?.trim(), width, height };
    })
  );
  expect(measurements).toHaveLength(5);
  for (const target of measurements) {
    expect(target.width, `${target.label} width`).toBeGreaterThanOrEqual(44);
    expect(target.height, `${target.label} height`).toBeGreaterThanOrEqual(44);
  }
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
