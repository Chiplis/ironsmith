import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Keep hand shares one row with auto-resolve and loop controls', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.route('https://**/*', route => route.fulfill({status:404,body:''}));
    for (const width of [1440, 1280, 1024]) {
      await page.setViewportSize({width, height:900});
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&keepHand&loopControls`);
      const dock = page.locator('[data-human-action-dock]');
      const keep = dock.getByRole('button', {name:'Keep hand',exact:true});
      await keep.waitFor({state:'visible'});
      await page.waitForTimeout(700);
      const main = await keep.boundingBox();
      const auto = await dock.locator('.battlefield-auto-pass-toggle').boundingBox();
      const loop = await dock.getByRole('button', {name:'Record loop',exact:true}).boundingBox();
      const bounds = await dock.boundingBox();
      assert.ok(auto.x >= main.x + main.width, 'auto-resolve is right of Keep hand');
      assert.ok(loop.x >= auto.x + auto.width, 'loop follows auto-resolve');
      for (const icon of [auto, loop]) {
        assert.ok(Math.abs(icon.y + icon.height / 2 - main.y - main.height / 2) < 2, 'icons share the main action row');
        assert.ok(icon.x + icon.width <= bounds.x + bounds.width + 1, 'icons fit inside the dock');
      }
    }
  } finally {await browser.close();await vite.close();}
});

test('long action labels shrink inside narrow buttons with quick controls', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1280,height:900}});
    await page.route('https://**/*', route => route.fulfill({status:404,body:''}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&motion&loopControls`);
    const dock = page.locator('[data-human-action-dock]');
    await dock.waitFor({state:'visible'});
    await page.getByRole('button',{name:'Next decision',exact:true}).click();
    await page.waitForTimeout(700);
    // Check the constrained button from the report and refit after resizing.
    for (const width of [1280, 1440]) {
      await page.setViewportSize({width, height:900});
      await page.waitForTimeout(350);
      const metrics = await dock.locator('.action-strip-main-title-text').first().evaluate(node => {
        const range = document.createRange();
        range.selectNodeContents(node);
        const text = range.getBoundingClientRect();
        const button = node.closest('.action-strip-main-region').getBoundingClientRect();
        return {text:{left:text.left,right:text.right,top:text.top,bottom:text.bottom},button:{left:button.left,right:button.right,top:button.top,bottom:button.bottom},font:parseFloat(getComputedStyle(node).fontSize),label:node.textContent};
      });
      assert.match(metrics.label, /Declare attackers/i);
      assert.ok(metrics.text.left >= metrics.button.left && metrics.text.right <= metrics.button.right, `text fits horizontally: ${JSON.stringify(metrics)}`);
      assert.ok(metrics.text.top >= metrics.button.top && metrics.text.bottom <= metrics.button.bottom, `text fits vertically: ${JSON.stringify(metrics)}`);
      assert.ok(metrics.font <= 14, 'label never exceeds the preferred size');
      if (metrics.button.right - metrics.button.left <= 110) assert.ok(metrics.font < 14, 'constrained label shrinks');
    }
    await dock.screenshot({path:'/tmp/ironsmith-action-label-fit.png'});
  } finally {await browser.close();await vite.close();}
});

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
      // The persistent mana row now reserves space below the decision panel.
      assert.ok(await scroll.evaluate(el=>el.clientHeight)>160,'multiple choices remain visible above the mana row');
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

test('desktop utilities sit below navigation and active mana stays inside the decision dock at double size', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    for (const width of [1600, 1280]) {
      await page.setViewportSize({width,height:900});
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&utilities&activeMana`);
      await page.locator('[data-human-action-dock]').waitFor({state:'visible'});
      await page.waitForTimeout(500);
      const navigation = await page.locator('.home-navigation').boundingBox();
      const utilities = await page.locator('.navigation-utility-controls').boundingBox();
      assert.ok(utilities.y >= navigation.y + navigation.height);
      const panel = await page.locator('.battlefield-human-decision-dock').boundingBox();
      const mana = await page.locator('.decision-mana-dock').boundingBox();
      assert.ok(mana.y >= panel.y + panel.height);
      const dock = await page.locator('[data-human-action-dock]').boundingBox();
      const symbols = await page.locator('.decision-mana-dock img').evaluateAll(nodes => nodes.map(n=>n.getBoundingClientRect().toJSON()));
      assert.equal(symbols.length, 6);
      for (const symbol of symbols) {
        assert.equal(symbol.width, 24);
        assert.equal(symbol.height, 24);
        assert.ok(symbol.x >= dock.x - 1 && symbol.right <= dock.x + dock.width + 1);
        assert.ok(symbol.bottom <= 900);
      }
      assert.equal(await page.locator('.local-player-mana-dock').count(), 0);
    }
  } finally {await browser.close();await vite.close();}
});

test('phase buttons keep their size and decision relayouts respect motion preferences', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1440,height:900}});
    await page.route('https://**/*', route => route.fulfill({status:404,body:''}));
    for (const reducedMotion of ['no-preference', 'reduce']) {
      await page.emulateMedia({reducedMotion});
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&utilities&motion`);
      const dock = page.locator('[data-human-action-dock]');
      await dock.waitFor({state:'visible'});
      await page.waitForTimeout(600);
      const labels = new Set();
      for (let step=0; step<3; step++) {
        const button=dock.locator('.decision-main-button').first();
        const bounds=await button.boundingBox();
        assert.equal(Math.round(bounds.width),190);
        assert.equal(Math.round(bounds.height),40);
        const panel = await dock.locator('.priority-inline-panel').boundingBox();
        assert.ok(bounds.x >= panel.x - 1 && bounds.x + bounds.width <= panel.x + panel.width + 1, 'fixed button is not clipped by its wrapper');
        labels.add(await button.innerText());
        await page.getByRole('button',{name:'Next decision',exact:true}).click();
        if(step<2) await page.waitForTimeout(350);
      }
      assert.ok(labels.size>1,'fixture exercises different phase labels');
      const animations=await dock.evaluate(n=>n.getAnimations().filter(a=>a.playState==='running').length);
      assert.equal(animations>0,reducedMotion==='no-preference','relayout animates only when motion is enabled');
      // Reverse an expansion before it completes, then verify the final geometry.
      await page.getByRole('button',{name:'Next decision',exact:true}).click();
      await page.waitForTimeout(650);
      assert.equal(await dock.evaluate(n=>n.getAnimations().filter(a=>a.playState==='running').length),0);
      const button=await dock.locator('.decision-main-button').first().boundingBox();
      assert.equal(Math.round(button.width),190);
      assert.equal(Math.round(button.height),40);
    }
  } finally {await browser.close();await vite.close();}
});
