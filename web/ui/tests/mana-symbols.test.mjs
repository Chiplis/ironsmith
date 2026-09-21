import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('localized grouped mana renders as individual symbols in rules text and costs', { timeout: 60000 }, async () => {
  const bucket = JSON.parse(await readFile(new URL('../public/card-i18n/es/by-name/py.json', import.meta.url), 'utf8'));
  const text = bucket['pyretic-ritual'].oracleText;
  assert.equal(text, 'Agrega {RRR}.');
  const server = await createServer({
    root: fileURLToPath(new URL('../', import.meta.url)),
    server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent',
  });
  let browser;
  try {
    await server.listen();
    browser = await chromium.launch();
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/mana-symbols.html`);
    await page.locator('#translated img').first().waitFor();
    assert.equal(await page.locator('#translated img').count(), 3);
    assert.deepEqual(
      await page.locator('#translated img').evaluateAll(nodes => nodes.map(node => node.outerHTML)),
      await page.locator('#canonical img').evaluateAll(nodes => nodes.map(node => node.outerHTML))
    );
    assert.equal(await page.locator('#translated').textContent(), 'Agrega .');
    assert.equal(await page.locator('#mixed em img').count(), 8);
    assert.equal(await page.locator('#cost svg, #cost img').count(), 7);
    assert.equal(await page.locator('#cost img').count(), 7, 'all supported symbols use local Mana SVGs');
    assert.equal(await page.locator('#cost img[src$="/12.svg"]').count(), 1, 'numeric mana remains one symbol');
    assert.ok(await page.locator('img').evaluateAll(nodes => nodes.every(node => node.complete && node.naturalWidth > 0)));
  } finally {
    await browser?.close();
    await server.close();
  }
});
