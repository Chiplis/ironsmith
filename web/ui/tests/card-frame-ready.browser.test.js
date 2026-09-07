import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('card frames reveal only after their hover delay, assets, and layout are ready', {timeout:60000}, async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-ready.html`);
    await page.getByRole('button', {name:'Run readiness checks'}).click();
    await page.waitForFunction(() => /ALL CHECKS PASSED|FAIL /.test(document.querySelector('[aria-label="Readiness results"]').textContent), null, {timeout:45000});
    const results = await page.getByLabel('Readiness results').textContent();
    assert.ok(results.includes('ALL CHECKS PASSED'), results);
    assert.equal(results.match(/^PASS /gm)?.length, 9, results);
  } finally {await browser.close();await vite.close();}
});
