import AxeBuilder from '@axe-core/playwright';
import { expect, test, type Download, type Page } from '@playwright/test';

async function downloadText(download: Download): Promise<string> {
  const stream = await download.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream) chunks.push(Buffer.from(chunk));
  return Buffer.concat(chunks).toString('utf8');
}

async function importOneResponse(page: Page, label = 'Roster 12', excerpt = 'A clear claim with one example.') {
  await page.getByRole('button', { name: /Add (my )?responses/ }).first().click();
  await page.getByLabel('Response text').fill(`# ${label}\n${excerpt}`);
  await page.getByRole('button', { name: 'Add to queue' }).click();
  await expect(page.getByRole('heading', { name: label })).toBeVisible();
}

async function expectNoSeriousAxeFindings(page: Page) {
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations.filter((issue) => ['serious', 'critical'].includes(issue.impact ?? ''))).toEqual([]);
}

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
  await page.reload();
});

test('@claim:demo-isolation keeps sample changes out of the real workspace', async ({ page }) => {
  await importOneResponse(page, 'Real roster 05', 'This is the teacher’s real local response.');
  await page.getByRole('button', { name: 'Try it with sample data' }).click();
  await expect(page).toHaveURL(/\/demo$/);
  await expect(page.getByText('Demo — sample data, nothing is saved to your real workspace')).toBeVisible();
  await expect(page.getByText('1 / 3 ready')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Roster 14' })).toBeVisible();
  await page.getByLabel(/Feedback draft/).fill('Changed only in the sample workspace.');
  await page.getByRole('button', { name: 'Reset demo' }).click();
  await expect(page.getByLabel(/Feedback draft/)).toHaveValue(/concrete details create urgency/);
  await page.getByRole('button', { name: 'Start for real' }).first().click();
  await expect(page.getByRole('heading', { name: 'Real roster 05' })).toBeVisible();
  await expect(page.getByLabel(/Feedback draft/)).toHaveValue('');
  await expect.poll(() => page.evaluate(() => localStorage.getItem('demo:rcq_workspace:v1'))).toBeNull();
});

test('@claim:text-import imports a text file and rejects a file over 1 MB', async ({ page }) => {
  await page.getByRole('button', { name: 'Add my responses' }).click();
  await page.getByLabel('Choose .txt file').setInputFiles({ name: 'class.txt', mimeType: 'text/plain', buffer: Buffer.from('# Roster 31\nA file-based response.') });
  await page.getByRole('button', { name: 'Add to queue' }).click();
  await expect(page.getByRole('heading', { name: 'Roster 31' })).toBeVisible();
  await page.getByRole('button', { name: 'Add responses' }).click();
  await page.getByLabel('Choose .txt file').setInputFiles({ name: 'too-large.txt', mimeType: 'text/plain', buffer: Buffer.alloc(1_000_001, 65) });
  await expect(page.getByRole('alert')).toContainText('over 1 MB');
});

test('@claim:teacher-review requires edited feedback and a personal next step', async ({ page }) => {
  await page.goto('/demo');
  await page.getByRole('button', { name: /Roster 21/ }).click();
  await page.locator('#criterion').selectOption('Whole response');
  await page.getByRole('button', { name: /Name the strength/ }).click();
  await page.getByLabel(/Feedback draft/).fill('Your window pattern is clear. Explain how the last open window changes Luis’s choice.');
  await page.getByRole('button', { name: /Save and next/ }).click();
  await expect(page.getByLabel(/One personal next step/)).toBeFocused();
  await page.getByLabel(/One personal next step/).fill('Connect the final window image to the brothers’ conversation.');
  await page.keyboard.press('Control+Enter');
  await expect(page.getByText('2 / 3 ready')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Roster 14' })).toBeVisible();
});

test('@claim:copy-feedback copies the complete reviewed comment', async ({ page, context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto('/demo');
  await page.getByRole('button', { name: /Copy feedback/ }).click();
  const clipboard = await page.evaluate(() => navigator.clipboard.readText());
  expect(clipboard).toContain('concrete details create urgency');
  expect(clipboard).toContain('Next step: Replace “started”');
});

test('@claim:csv-export exports one data row for each sample response', async ({ page }) => {
  await page.goto('/demo');
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export all CSV' }).click();
  const contents = await downloadText(await pending);
  const records = contents.split('\r\n');
  expect(records).toHaveLength(4);
  expect(records[0]).toBe('"Response","Criterion","Feedback","Status"');
  expect(contents).toContain('"Roster 08"');
  expect(contents).toContain('"Roster 14"');
  expect(contents).toContain('"Roster 21"');
});

test('@claim:local-backup downloads a restorable workspace document', async ({ page }) => {
  await page.goto('/demo');
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download local backup' }).click();
  const saved = JSON.parse(await downloadText(await pending));
  expect(saved.version).toBe(1);
  expect(saved.submissions.map((item: { label: string }) => item.label)).toEqual(['Roster 08', 'Roster 14', 'Roster 21']);
  expect(saved.comments.some((item: { title: string }) => item.title === 'Address another view')).toBe(true);
});

test('@claim:local-autosave restores teacher edits after reload', async ({ page }) => {
  await importOneResponse(page, 'Reload check', 'A response that will remain in this browser.');
  await page.getByLabel(/Feedback draft/).fill('Keep this teacher-written sentence after reload.');
  await page.getByLabel(/One personal next step/).fill('Add one cited example.');
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Reload check' })).toBeVisible();
  await expect(page.getByLabel(/Feedback draft/)).toHaveValue('Keep this teacher-written sentence after reload.');
  await expect(page.getByLabel(/One personal next step/)).toHaveValue('Add one cited example.');
});

test('@claim:offline-reload reloads the sample queue without a network', async ({ browser }) => {
  const context = await browser.newContext({ serviceWorkers: 'allow' });
  const page = await context.newPage();
  await page.goto('/demo');
  await page.evaluate(async () => { await navigator.serviceWorker.ready; });
  if (!(await page.evaluate(() => Boolean(navigator.serviceWorker.controller)))) await page.reload();
  await context.setOffline(true);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.getByText('Offline · local save on')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Roster 14' })).toBeVisible();
  await context.setOffline(false);
  await context.close();
});

test('@claim:student-text-private sends no student excerpt during the demo workflow', async ({ page, context }) => {
  const requests: { url: string; body: string }[] = [];
  page.on('request', (request) => requests.push({ url: request.url(), body: request.postData() ?? '' }));
  await page.goto('/demo');
  await page.getByLabel(/Feedback draft/).fill('A teacher edit that also stays local.');
  await page.getByRole('button', { name: /Roster 21/ }).click();
  await page.waitForTimeout(100);
  const origin = new URL(page.url()).origin;
  expect(requests.every((request) => new URL(request.url).origin === origin)).toBe(true);
  expect(requests.map((request) => request.body).join('\n')).not.toContain('train doors closed');
  expect(requests.map((request) => request.body).join('\n')).not.toContain('teacher edit');
  expect(await context.cookies()).toEqual([]);
});

test('@claim:no-automatic-feedback leaves a new response blank for the teacher', async ({ page }) => {
  await importOneResponse(page, 'No automation', 'An original response that must not be scored or rewritten.');
  await expect(page.getByLabel(/Feedback draft/)).toHaveValue('');
  await expect(page.getByLabel(/One personal next step/)).toHaveValue('');
  await expect(page.locator('.review-sheet')).not.toContainText(/score:\s*\d|generated feedback/i);
});

test('@claim:free-core completes review and export without an account or checkout', async ({ page }) => {
  const externalRequests: string[] = [];
  page.on('request', (request) => {
    if (new URL(request.url()).origin !== new URL(page.url()).origin) externalRequests.push(request.url());
  });
  await page.goto('/demo');
  await page.getByRole('button', { name: /Save and next/ }).click();
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export all CSV' }).click();
  expect((await downloadText(await pending)).split('\r\n')).toHaveLength(4);
  await expect(page.getByRole('link', { name: /buy|checkout/i })).toHaveCount(0);
  expect(externalRequests).toEqual([]);
});

test('@claim:aggregate-pageview loads without analytics scripts or tracking cookies', async ({ page, context }) => {
  const scripts: string[] = [];
  const posts: string[] = [];
  page.on('request', (request) => {
    if (request.resourceType() === 'script') scripts.push(request.url());
    if (request.method() === 'POST') posts.push(new URL(request.url()).pathname);
  });
  await page.goto('/');
  await expect.poll(() => posts.filter((path) => path === '/api/pageview').length).toBe(1);
  expect(scripts.every((url) => new URL(url).origin === new URL(page.url()).origin)).toBe(true);
  expect(await context.cookies()).toEqual([]);
});

test('@claim:delete-response removes only the confirmed response', async ({ page }) => {
  await page.goto('/demo');
  page.once('dialog', (dialog) => dialog.dismiss());
  await page.getByRole('button', { name: 'Delete' }).click();
  await expect(page.getByRole('heading', { name: 'Roster 14' })).toBeVisible();
  page.once('dialog', (dialog) => dialog.accept());
  await page.getByRole('button', { name: 'Delete' }).click();
  await expect(page.getByRole('heading', { name: 'Roster 14' })).toHaveCount(0);
  await expect(page.getByText(/1 \/ 2 ready/)).toBeVisible();
});

test('@claim:health-build reports a usable build identity', async ({ request }) => {
  const response = await request.get('/health');
  expect(response.ok()).toBe(true);
  const health = await response.json();
  expect(health.status).toBe('ok');
  expect(health.build_sha).toMatch(/^(development|[0-9a-f]{7,40})$/);
  expect(health.build_sha).not.toBe('unknown');
});

test('uses plain route titles, browser history, and a designed HTTP 404', async ({ page, request }) => {
  await expect(page).toHaveTitle('Rubric Comment Queue — review writing feedback');
  await page.getByRole('link', { name: 'Privacy', exact: true }).first().click();
  await expect(page).toHaveTitle('Privacy — Rubric Comment Queue');
  await expect(page.getByRole('heading', { name: 'Privacy for your feedback queue' })).toBeFocused();
  await page.goBack();
  await expect(page).toHaveTitle('Rubric Comment Queue — review writing feedback');
  const missing = await request.get('/not-a-real-page');
  expect(missing.status()).toBe(404);
  expect(await missing.text()).toContain('<h1>Page not found</h1>');
});

test('has no serious accessibility findings across app, dialog, demo, legal, and dark states', async ({ page }) => {
  await expectNoSeriousAxeFindings(page);
  await page.getByRole('button', { name: 'Add my responses' }).click();
  await expectNoSeriousAxeFindings(page);
  await page.getByRole('button', { name: 'Close import dialog' }).click();
  await page.getByRole('button', { name: 'Use dark theme' }).click();
  await page.goto('/demo');
  await expectNoSeriousAxeFindings(page);
  await page.goto('/privacy');
  await expectNoSeriousAxeFindings(page);
  await page.goto('/terms');
  await expectNoSeriousAxeFindings(page);
});

test('dialog close and cancel controls restore focus', async ({ page }) => {
  const importOpener = page.getByRole('button', { name: 'Add my responses' });
  await importOpener.focus();
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: 'Close import dialog' }).click();
  await expect(importOpener).toBeFocused();
  await importOpener.click();
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(importOpener).toBeFocused();

  await page.goto('/demo');
  const commentOpener = page.getByRole('button', { name: 'Add a comment block' });
  await commentOpener.focus();
  await page.keyboard.press('Enter');
  await page.getByRole('button', { name: 'Close comment dialog' }).click();
  await expect(commentOpener).toBeFocused();
});

test('recovers from malformed workspace and license caches', async ({ page }) => {
  const errors: Error[] = [];
  page.on('pageerror', (error) => errors.push(error));
  await page.evaluate(() => {
    localStorage.setItem('rcq_workspace:v1', '{bad workspace');
    localStorage.setItem('sb_license_verdict:rubric-comment-queue', '{bad license');
  });
  await page.reload();
  await expect(page.locator('main')).toHaveAttribute('aria-busy', 'false');
  await expect(page.getByRole('button', { name: 'Add my responses' })).toBeVisible();
  expect(errors).toEqual([]);
});

test('all visible mobile controls meet 44px targets and body text remains 17px', async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== 'mobile', 'Measured at the mobile breakpoint.');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/demo');
  await page.getByRole('button', { name: /Roster 08/ }).click();
  const undersized = await page.locator('a, button, select, textarea, label.file-button').evaluateAll((elements) => elements
    .filter((element) => {
      const style = getComputedStyle(element);
      const box = element.getBoundingClientRect();
      return style.visibility !== 'hidden' && style.display !== 'none' && box.width > 0 && box.height > 0;
    })
    .map((element) => {
      const box = element.getBoundingClientRect();
      return { name: element.getAttribute('aria-label') ?? element.textContent?.trim(), width: box.width, height: box.height };
    })
    .filter((box) => box.width < 44 || box.height < 44));
  expect(undersized).toEqual([]);
  expect(await page.locator('body').evaluate((body) => parseFloat(getComputedStyle(body).fontSize))).toBeGreaterThanOrEqual(17);
  await page.goto('/privacy');
  const back = await page.getByRole('link', { name: /Back to the queue/ }).boundingBox();
  expect(back?.height).toBeGreaterThanOrEqual(44);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(390);
});

test('keyboard skip, reduced motion, and focus validation remain usable', async ({ page }) => {
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeFocused();
  await page.keyboard.press('Enter');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  expect(await page.evaluate(() => matchMedia('(prefers-reduced-motion: reduce)').matches)).toBe(true);
  const duration = await page.getByRole('button', { name: 'Try it with sample data' }).evaluate((button) => getComputedStyle(button).transitionDuration);
  expect(parseFloat(duration)).toBeLessThan(0.01);
});

test('@claim:rate-limit separates forwarded clients and sends Retry-After', async ({ request }) => {
  const burst = await Promise.all(Array.from({ length: 80 }, () => request.get('/api/backup', { headers: { 'X-Forwarded-For': '203.0.113.8' } })));
  const limited = burst.filter((response) => response.status() === 429);
  expect(limited.length).toBeGreaterThan(0);
  expect(limited.every((response) => response.headers()['retry-after'] !== undefined)).toBe(true);
  const other = await request.get('/api/backup', { headers: { 'X-Forwarded-For': '203.0.113.9' } });
  expect(other.status()).toBe(401);
});
