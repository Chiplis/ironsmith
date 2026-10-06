import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('multiplayer surrender replaces card creation and confirms through red Yes / green No', { timeout: 60000 }, async t => {
  const server = await createServer({ root, logLevel: 'error', server: { host: '127.0.0.1', port: 0, hmr: false } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  for (const mode of ['trusted', 'verified']) {
    for (const waiting of [false, true]) {
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/surrender-controls.html?mode=${mode}${waiting ? '&waiting=1' : ''}`);
      const surrender = page.getByRole('button', { name: 'Surrender', exact: true });
      await surrender.click();
      assert.equal(await page.getByRole('button', { name: /Add Card|Compile Card/ }).count(), 0);
      const yes = page.getByRole('button', { name: 'Yes', exact: true });
      const no = page.getByRole('button', { name: 'No', exact: true });
      assert.equal(await yes.evaluate(el => getComputedStyle(el).getPropertyValue('--decision-main-accent').trim()), 'rgb(255, 133, 133)');
      assert.equal(await no.evaluate(el => getComputedStyle(el).getPropertyValue('--decision-main-accent').trim()), 'rgb(133, 239, 172)');
      assert.equal(await yes.evaluate(el => getComputedStyle(el).color), 'rgb(255, 133, 133)');
      assert.equal(await no.evaluate(el => getComputedStyle(el).color), 'rgb(133, 239, 172)');
      if (mode === "verified" && !waiting) await page.screenshot({ path: "/tmp/ironsmith-surrender-confirmation.png" });
      await no.click();
      assert.equal(await yes.count(), 0);
      assert.equal(await page.locator('output').textContent(), '[]');
      await surrender.click();
      await yes.click();
      await page.waitForFunction(() => document.querySelector('output').textContent !== '[]');
      assert.deepEqual(JSON.parse(await page.locator('output').textContent()), [{ type: 'forfeit_player', player: 1, reason: 'surrender' }]);
    }
  }
  assert.deepEqual(errors, []);
});
