import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

for (const layout of ['default', 'mobile-battle-top', 'mobile-battle-bottom']) {
test(`battlefield ${layout} geometry survives removal, activation and board growth`, async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:layout === 'default' ? 1280 : 430,height:900}});
    const errors = [];
    page.on('pageerror', error => errors.push(String(error)));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/battlefield-stable-slots.html?layout=${layout}`);
    const expectObjects = async expected => {
      await page.waitForFunction(ids => {
        const actual = new Set([...document.querySelectorAll('.battlefield-row-card[data-member-stable-ids]')]
          .flatMap(node => String(node.dataset.memberStableIds).split(',').map(Number)));
        return actual.size === ids.length && ids.every(id => actual.has(id));
      }, expected);
      const rects = await page.locator('.battlefield-row-card[data-member-stable-ids]').evaluateAll(nodes =>
        nodes.map(node => { const r = node.getBoundingClientRect(); return {x:r.x,y:r.y,width:r.width,height:r.height}; }));
      assert.ok(rects.length > 0);
      assert.ok(rects.every(r => Object.values(r).every(Number.isFinite) && r.width > 0 && r.height > 0));
    };
    const initial = [1,2,3,4,5,6,7,8];
    await expectObjects(initial);
    await page.getByRole('button', {name:'Activate source', exact:true}).click();
    await expectObjects(initial);
    await page.getByRole('button', {name:'Cancel activation', exact:true}).click();
    await page.getByRole('button', {name:'Remove object', exact:true}).click();
    await expectObjects(initial.filter(id => id !== 2));
    await page.getByRole('button', {name:'Add many objects', exact:true}).click();
    await expectObjects([...initial.filter(id => id !== 2), ...Array.from({length:55}, (_,i) => i + 20)]);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
}
