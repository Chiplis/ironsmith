import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('decision fits beside hand and zones, and overflowing targets scroll', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1440,height:900}});
    for (const width of [1440, 1280]) {
      await page.setViewportSize({width,height:900});
      for (const options of [2, 30]) {
        await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&options=${options}`);
        const dock = page.locator('[data-human-action-dock]');
        await dock.waitFor({state:'visible'});
        await page.waitForTimeout(900);
        const bounds = await dock.boundingBox();
        const cards = await page.locator('.hand-layout-item').evaluateAll(nodes => nodes.map(n=>n.getBoundingClientRect().right));
        assert.ok(cards.length > 0, 'fixture includes the desktop hand');
        const handTop=await page.locator('.hand-layout-item .game-card').evaluateAll(nodes=>Math.min(...nodes.map(n=>n.getBoundingClientRect().top)));
        assert.ok(bounds.x >= Math.max(...cards) || bounds.y+bounds.height<=handTop, 'dock clears the hand');
        const zones = await page.locator('[data-local-zone-piles="true"] > .zone-pile-slot').evaluateAll(nodes => nodes.map(n=>n.getBoundingClientRect().left));
        assert.ok(bounds.x + bounds.width <= Math.min(...zones), 'dock stops before zones');
        const list = dock.locator('.decision-strip-scroll--vertical-target-options');
        if (options === 2) {
          await dock.getByRole('button', {name:'Alice',exact:true}).click();
          await dock.getByRole('button', {name:'Bob',exact:true}).last().click();
          await page.waitForTimeout(300);
        }
        const metrics = await list.evaluate(n=>({client:n.clientHeight,scroll:n.scrollHeight}));
        if (options === 2) assert.ok(metrics.scroll <= metrics.client + 1, `short list fits: ${JSON.stringify(metrics)}`);
        else {
          assert.ok(metrics.scroll > metrics.client, 'long list overflows');
          await list.hover();
          await page.mouse.wheel(0, 300);
          await page.waitForTimeout(200);
          assert.ok(await list.evaluate(n=>n.scrollTop) > 0, 'wheel moves the target list');
          assert.equal(await list.locator('div').evaluateAll(nodes=>nodes.filter(n=>['auto','scroll'].includes(getComputedStyle(n).overflowY)).length), 0, 'no nested scroll traps');
        }
      }
    }
  } finally { await browser.close(); await vite.close(); }
});


test('compact object choices use the height beside the deck and clear the visible hand', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.route('https://**/*', route => route.fulfill({status:404,body:''}));
    for (const viewport of [{width:1710,height:666}, {width:1280,height:800}]) {
      await page.setViewportSize(viewport);
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&objects&hand=7&utilities`);
      const dock = page.locator('[data-human-action-dock]');
      await dock.waitFor({state:'visible'});
      await page.waitForTimeout(800);
      const bounds = await dock.boundingBox();
      const hand = await page.locator('.hand-layout-item .game-card').evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect();return {right:r.right,top:r.top};}));
      assert.ok(hand.length > 0, "fixture includes visible hand cards");
      assert.ok(bounds.y+bounds.height<=viewport.height-14, "whole dock stays in viewport");
      assert.ok(hand.every(card=>bounds.x>=card.right || bounds.y+bounds.height<=card.top), `hand collision: ${JSON.stringify({bounds,hand})}`);
      const zones = await page.locator('[data-local-zone-piles="true"] > .zone-pile-slot').evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect();return {left:r.left,top:r.top};}));
      assert.ok(bounds.x+bounds.width<=Math.min(...zones.map(r=>r.left))-10,'right edge remains left of zones');
      const topLimit=Math.max(48,Math.min(...zones.map(r=>r.top)));
      assert.ok(bounds.y>=topLimit-2 && bounds.y<=topLimit+60, `long choices expand to deck level: ${JSON.stringify({bounds,topLimit})}`);
      const scroll=dock.locator('.decision-strip-scroll');
      assert.ok(await scroll.evaluate(el=>el.clientHeight)>180,'multiple choices visible before scrolling');
      await page.screenshot({path:`/tmp/ironsmith-decision-${viewport.width}.png`});
      await page.locator('.hand-layout-item .game-card').last().hover({force:true});
      await page.waitForTimeout(500);
      const hoveredBounds = await dock.boundingBox();
      await page.screenshot({path:`/tmp/ironsmith-hover-${viewport.width}.png`});
      const hoveredHand = await page.locator('.hand-layout-item .game-card').evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect();return {right:r.right,top:r.top};}));
      assert.ok(hoveredHand.every(card=>hoveredBounds.x>=card.right || hoveredBounds.y+hoveredBounds.height<=card.top), `enlarging a hand card cannot cover decision controls: ${JSON.stringify({viewport,hoveredBounds,hoveredHand})}`);
      const submit = await dock.locator('.decision-stack-footer .action-strip-submit-button').boundingBox();
      assert.ok(submit.x >= hoveredBounds.x && submit.x + submit.width <= hoveredBounds.x + hoveredBounds.width, 'Submit stays inside narrow panels');
    }
  } finally {await browser.close();await vite.close();}
});
