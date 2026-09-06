import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('zone piles align, scroll, animate and require a separate target click', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1200,height:800}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles.html`);
    const pile = page.locator('[data-zone-pile="graveyard"]');
    await pile.waitFor();
    await page.waitForTimeout(300);
    const pileBox = await pile.boundingBox();
    const cardBox = await page.locator('.battlefield-row-card').boundingBox();
    assert.ok(pileBox.width < cardBox.width);
    const labelBox = await page.locator('.zone-pile-label').first().boundingBox();
    assert.ok(labelBox.y + labelBox.height <= pileBox.y);
    assert.equal(await pile.locator('.zone-pile-label').count(), 0);
    assert.ok(Math.abs(pileBox.y-cardBox.y) < 2);
    assert.ok(pileBox.x > cardBox.x);
    const exileBox = await page.locator('[data-zone-pile="exile"]').boundingBox();
    assert.ok(Math.abs(exileBox.x-pileBox.x)<1);
    assert.ok(exileBox.y >= pileBox.y+pileBox.height);
    // Browsing your own zone works before entering targeting mode.
    await pile.click();
    await page.keyboard.press('Escape');
    await page.locator('.zone-pile-menu').waitFor({state:'hidden'});
    await page.getByRole('button',{name:'Toggle targeting'}).click();
    assert.equal(await pile.getAttribute('data-has-targets'),'true');
    await page.waitForFunction(() => getComputedStyle(document.querySelector('[data-zone-pile=graveyard]')).borderTopColor === 'rgb(255, 255, 255)');
    await pile.click();
    assert.equal(await page.locator('output').textContent(),'none');
    const menu = page.locator('.zone-pile-menu');
    await menu.waitFor();
    assert.equal(await menu.locator('header').count(),0);
    await page.waitForTimeout(300);
    const menuBox = await menu.boundingBox();
    const fieldBox = await page.locator('.has-zone-piles').boundingBox();
    assert.ok(Math.abs(menuBox.x - fieldBox.x) <= 2);
    const expandedCard = await page.locator('.zone-pile-card-row').first().boundingBox();
    assert.ok(Math.abs(expandedCard.width - pileBox.width) < 1);
    assert.ok(expandedCard.width < cardBox.width);
    const rowBoxes = await page.locator('.zone-pile-card-row').evaluateAll(rows=>rows.slice(0,2).map(row=>({x:row.getBoundingClientRect().x,y:row.getBoundingClientRect().y})));
    assert.equal(rowBoxes[0].y,rowBoxes[1].y);
    assert.ok(rowBoxes[1].x > rowBoxes[0].x);
    assert.equal(await menu.evaluate(el=>getComputedStyle(el).animationDuration),'0.22s');
    assert.deepEqual(await page.locator('.zone-pile-card-row').evaluateAll(rows=>rows.slice(0,3).map(row=>row.dataset.objectId)),['20','19','18']);
    assert.ok(await page.locator('.zone-pile-card-list').evaluate(el=>el.scrollWidth>el.clientWidth));
    assert.equal(await page.locator('.zone-pile-card-row:disabled').count(),19);
    await page.locator('.zone-pile-card-row[data-object-id="20"]').click();
    assert.equal(await page.locator('output').textContent(),'20');
    await menu.waitFor({state:'hidden'});
    await page.setViewportSize({width:600,height:800});
    await pile.click();
    await page.waitForTimeout(300);
    await page.screenshot({path:'/tmp/zone-piles-verified.png'});
  } finally { await browser.close(); await vite.close(); }
});


test('all four players can browse piles and select a graveyard target on the full table', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1600,height:1000}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html`);
    await page.locator('[data-zone-pile]').first().waitFor();
    assert.equal(await page.locator('[data-zone-pile]').count(),8);
    await page.waitForTimeout(350);
    const stack = await page.locator('.my-zone-stack-rail').boundingBox();
    const field = await page.locator('[data-my-zone] .my-zone-board-shell').boundingBox();
    const topCard = await page.locator('[data-my-zone] .battlefield-row-card').evaluateAll(cards=>Math.min(...cards.map(card=>card.getBoundingClientRect().top)));
    assert.ok(Math.abs(stack.y-topCard)<2);
    assert.ok(Math.abs(stack.x-field.x)<2);
    await page.screenshot({path:'/tmp/stack-left-zones-right-verified.png'});
    for(const owner of ['0','1','2','3']) {
      await page.locator(`[data-zone-pile="graveyard"][data-zone-owner="${owner}"]`).click();
      await page.keyboard.press('Escape');
      await page.locator('.zone-pile-menu').waitFor({state:'hidden'});
    }
    await page.evaluate(()=>{
      window.zoneTargetEvents=[];
      window.addEventListener('ironsmith:target-choice',event=>window.zoneTargetEvents.push(event.detail.target));
    });
    await page.getByRole('button',{name:'Target graveyard cards'}).click();
    const opponentPile=page.locator('[data-zone-pile="graveyard"][data-zone-owner="1"]');
    assert.equal(await opponentPile.getAttribute('data-has-targets'),'true');
    await opponentPile.click();
    assert.deepEqual(await page.evaluate(()=>window.zoneTargetEvents),[]);
    await page.locator('.zone-pile-card-row[data-object-id="1001"]').click();
    assert.deepEqual(await page.evaluate(()=>window.zoneTargetEvents),[{kind:'object',object:1001}]);
  } finally { await browser.close(); await vite.close(); }
});

test('shortcut preview highlights and opens both off-battlefield zones while still at priority', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1200,height:800}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles.html`);
    await page.getByRole('button',{name:'Start shortcut targeting'}).click();
    for (const [zone, target] of [['graveyard','20'],['exile','31']]) {
      const pile = page.locator(`[data-zone-pile="${zone}"]`);
      await page.waitForFunction(zone => document.querySelector(`[data-zone-pile="${zone}"]`).dataset.hasTargets === 'true', zone);
      await pile.hover();
      await page.locator('.zone-pile-menu').waitFor();
      assert.equal(await page.locator(`.zone-pile-card-row[data-object-id="${target}"]`).getAttribute('data-target-legal'),'true');
      assert.equal(await page.locator('output').textContent(),'none');
      await page.mouse.move(900,700);
      await page.keyboard.press('Escape');
      await page.locator('.zone-pile-menu').waitFor({state:'hidden'});
    }
  } finally { await browser.close(); await vite.close(); }
});

test('cast choices portal above inspector and an open zone list and remain clickable', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1200,height:800}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles.html`);
    await page.locator('[data-zone-pile="graveyard"]').click();
    await page.getByRole('button',{name:'Show cast choices'}).evaluate(el=>el.click());
    const choices=page.locator('[data-action-popover]');
    await choices.waitFor();
    await page.waitForTimeout(300);
    assert.equal(await choices.evaluate(el=>el.parentElement === document.body),true);
    assert.ok(await choices.evaluate(el=>Number(getComputedStyle(el).zIndex)>Number(getComputedStyle(document.querySelector('.floating-card-preview')).zIndex)));
    assert.ok(await choices.evaluate(el=>Number(getComputedStyle(el).zIndex)>Number(getComputedStyle(document.querySelector('.zone-pile-menu')).zIndex)));
    await choices.locator('[role=button]').first().click();
    assert.equal(await page.locator('output').textContent(),'cast');
  } finally { await browser.close(); await vite.close(); }
});
