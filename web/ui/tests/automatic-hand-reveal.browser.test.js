import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('only the owner auto-reveals once after the interaction gate clears', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/automatic-hand-reveal.html`);
    const commands = async () => JSON.parse(await page.locator('output').textContent());
    await page.getByText('Owner', {exact:true}).click();
    await page.waitForTimeout(100);
    assert.deepEqual(await commands(), []);
    await page.getByText('Unblock', {exact:true}).click();
    await page.waitForFunction(() => JSON.parse(document.querySelector('output').textContent).length === 1);
    assert.deepEqual(await commands(), [{type:'select_objects',object_ids:[10,11,12,13,14,15,16]}]);
    await page.getByText('Rerender', {exact:true}).click();
    await page.waitForTimeout(100);
    assert.equal((await commands()).length, 1);
    await page.getByText('Manual', {exact:true}).click();
    await page.waitForTimeout(100);
    assert.equal((await commands()).length, 1);
    await page.getByText('Automatic', {exact:true}).click();
    await page.waitForFunction(() => JSON.parse(document.querySelector('output').textContent).length === 2);
  } finally { await browser.close(); await vite.close(); }
});
