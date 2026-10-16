import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('rules sources render distinct frames and live text without card artwork', {timeout:60000}, async () => {
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1240,height:460}});
    const errors=[];
    page.on('pageerror',error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/rules-source-frame.html`);
    await page.locator('[data-rules-source="dungeon_room"]').waitFor();
    for (const kind of ['initiative','monarch','ring','dungeon_room']) {
      const frame=page.locator(`[data-rules-source="${kind}"]`);
      assert.equal(await frame.count(),1);
      assert.ok(await frame.locator('.rules-source-emblem').isVisible());
      assert.equal(await frame.locator('img').count(),0);
    }
    assert.match(await page.locator('[data-rules-source="dungeon_room"]').innerText(),/Search\s+your library for a basic land card/);
    assert.deepEqual(errors,[]);
    await page.screenshot({path:'/tmp/ironsmith-rules-source-frames.png'});
  } finally {await browser.close();await vite.close();}
});
