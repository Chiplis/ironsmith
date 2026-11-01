import { chromium } from 'playwright';
import { createServer } from 'vite';
import { mkdir } from 'node:fs/promises';
const output = new URL('../../../output/optional-target-submit/', import.meta.url);
await mkdir(output, { recursive: true });
const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
await vite.listen();
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1100, height: 900 }, deviceScaleFactor: 2, reducedMotion: 'reduce' });
  await page.route('**/api.scryfall.com/**', route => route.abort());
  await page.route('**/cards.scryfall.io/**', route => route.abort());
  const open = async scenario => {
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/forced-target-auto-pick.html?scenario=${scenario}&capture=1&layout=strip`);
    await page.locator('.decision-option-row[aria-pressed]').first().waitFor();
  };
  const bob = page.locator('.decision-option-row[aria-pressed]').filter({ hasText: 'Bob' });
  const carol = page.locator('.decision-option-row[aria-pressed]').filter({ hasText: 'Carol' });
  const capture = async name => {
    await page.waitForTimeout(350);
    await page.locator('[data-target-preview]').screenshot({ path: new URL(name, output).pathname });
  };
  await open('optional-one');
  await bob.hover();
  await capture('one-target-hover.png');
  await open('optional-two');
  await bob.hover();
  await capture('two-targets-first-hover.png');
  await bob.click();
  await carol.hover();
  await capture('two-targets-second-hover.png');
  console.log(output.pathname);
} finally {
  await browser.close();
  await vite.close();
}
