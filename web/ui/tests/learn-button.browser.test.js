import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Learn records each click and shows a popup even if tracking fails', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const origin = `http://127.0.0.1:${vite.httpServer.address().port}`;
    await page.goto(`${origin}/tests/learn-button.html`);
    const learn = page.getByRole('button', {name:'Learn',exact:true});
    const responsePromise = page.waitForResponse(r=>r.url().includes('/events/learn-click.txt?'));
    await learn.click();
    const response = await responsePromise;
    assert.equal(response.status(),200);
    assert.equal(response.headers()['content-length'],'0');
    assert.equal(new URL(response.url()).origin,origin);
    await page.getByText('Coming soon!',{exact:true}).waitFor({state:'visible'});
    assert.equal(await page.locator('.learn-placeholder').count(),0);
    await page.keyboard.press('Escape');
    await page.getByText('Coming soon!',{exact:true}).waitFor({state:'hidden'});
    await page.route('**/events/learn-click.txt?*',route=>route.abort());
    const requestPromise = page.waitForRequest(r=>r.url().includes('/events/learn-click.txt?'));
    await learn.focus();
    await page.keyboard.press('Enter');
    const request = await requestPromise;
    assert.notEqual(request.url(),response.url(),'each click bypasses cached URLs');
    await page.getByText('Coming soon!',{exact:true}).waitFor({state:'visible'});
  } finally { await browser.close(); await vite.close(); }
});
