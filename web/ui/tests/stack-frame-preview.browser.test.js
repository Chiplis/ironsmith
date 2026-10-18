import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
for (const viewport of [{ width: 1440, height: 900 }, { width: 844, height: 390 }]) test(`automatic stack frame survives disappearing sources and closes at priority (${viewport.width}px)`, { timeout: 60000 }, async () => {
 const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
 await vite.listen();
 const browser = await chromium.launch();
 try {
  const page = await browser.newPage({ viewport });
  // The automatic frame belongs to the rail: the "Full cards" appearance.
  // ("Compact artwork" shows the stack as a pile of full-size frames.)
  await page.addInitScript(() => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({ compactCards: false })));
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.route('https://**scryfall**/**', route => route.fulfill({ status:404, body:'' }));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-frame-preview.html`);
  await page.waitForFunction(() => window.setStackState);
  await page.evaluate(() => window.setStackState({ ...window.stackFixture.initial, stack_objects:[window.stackFixture.entry] }));
  const frame = page.locator('[data-automatic-stack-preview="true"][data-visible="true"]');
  await frame.waitFor();
  assert.match(await frame.innerText(), /Resolving source/);
  assert.doesNotMatch(await frame.innerText(), /Wrong card/);
  // Match the normal hover size and stay beside either screen-edge stack.
  await page.waitForFunction(() => {
    const anchor = document.querySelector('[data-stack-preview-anchor]').getBoundingClientRect();
    const preview = document.querySelector('[data-visible="true"][data-automatic-stack-preview]')?.getBoundingClientRect();
    return preview && Math.abs(preview.left - anchor.right - 8) < 2;
  });
  const normalHeight = await page.evaluate(() => {
    const reference = document.createElement('aside');
    reference.className = 'floating-card-preview';
    document.body.append(reference);
    const height = parseFloat(getComputedStyle(reference).height);
    reference.remove();
    return height;
  });
  assert.ok(Math.abs((await frame.boundingBox()).height - normalHeight) < 2);
  await page.locator('[data-stack-preview-anchor]').evaluate(el => {
    el.style.left = 'auto'; el.style.right = '10px';
  });
  await page.waitForFunction(() => {
    const anchor = document.querySelector('[data-stack-preview-anchor]').getBoundingClientRect();
    const preview = document.querySelector('[data-visible="true"][data-automatic-stack-preview]')?.getBoundingClientRect();
    return preview && Math.abs(anchor.left - preview.right - 8) < 2;
  });

  if(viewport.width >= 1024) {
    await page.locator('#desktop-stack-fixture').evaluate(el=>el.style.display='block');
    await page.locator('[data-stack-preview-anchor]').evaluate(el=>el.style.top='100px');
    await page.waitForTimeout(600);
    await page.waitForFunction(()=>{
      const frame=document.querySelector('[data-automatic-stack-preview="true"][data-visible="true"]')?.getBoundingClientRect();
      const rail=document.querySelector('.my-zone-stack-rail').getBoundingClientRect();
      return frame && Math.abs(frame.left-rail.right-12)<2 && Math.abs(frame.top-rail.top)<2;
    });
    const bounds=await frame.boundingBox();
    assert.ok(bounds.height>300,'rail frame remains readable');
    assert.ok(bounds.y+bounds.height<=viewport.height-7,'frame fits screen');
    await page.locator('.my-zone-stack-rail .stack-timeline-entry').evaluate(el=>el.style.height='180px');
    await page.waitForTimeout(250);
    const expanded=await frame.boundingBox();
    assert.ok(Math.abs(expanded.x-bounds.x)<2 && Math.abs(expanded.y-bounds.y)<2,'expanding the stack entry leaves preview anchored to the rail');
    assert.ok(Math.abs(expanded.height-bounds.height)<2,'expanded stack text does not shrink the preview');
  }

  if (viewport.width >= 1024) {
    for (const id of [10, 20]) {
      const card = page.locator(`[data-field-card="${id}"]`);
      await card.hover();
      const inspector = page.locator(`[data-card-hover-preview][data-visible="true"]:not([data-automatic-stack-preview])[data-preview-object-id="${id}"]`);
      await inspector.waitFor();
      assert.match(await inspector.innerText(), id === 10 ? /Resolving source/ : /Wrong card/);
      assert.equal(await frame.count(), 1, 'stack frame remains while inspecting the battlefield');
      await card.click();
      await page.mouse.move(1400, 850);
      await inspector.waitFor();
      assert.equal(await frame.count(), 1, 'stack frame remains beside clicked field frame');
      await page.evaluate(() => window.clearFieldPin());
    }
  }

  await page.evaluate(() => window.setStackState({ players: [], stack_objects: [], resolving_stack_object: window.stackFixture.entry, decision:{kind:'select_options'} }));
  await frame.waitFor();
  assert.match(await frame.innerText(), /Draw a card/);
  assert.equal(await frame.evaluate(el => getComputedStyle(el).pointerEvents), 'none');
  await page.mouse.click(viewport.width - 5, viewport.height - 5);
  await page.waitForFunction(() => !document.querySelector('[data-card-hover-preview][data-visible="true"]'));
  // A refreshed snapshot of the same resolving entry stays dismissed.
  await page.evaluate(() => window.setStackState({ players: [], stack_objects: [], resolving_stack_object: { ...window.stackFixture.entry }, decision:{kind:'select_options'} }));
  await page.waitForTimeout(600);
  assert.equal(await frame.count(), 0);
  // The next entry automatically opens its frame.
  await page.evaluate(() => window.setStackState({ players: [], stack_objects: [{ ...window.stackFixture.entry, id: 40 }], decision:{kind:'priority'} }));
  await frame.waitFor();

  await page.evaluate(() => window.setStackState({players:[],stack_objects:[],resolving_stack_object:window.stackFixture.entry,decision:{kind:'priority'}}));
  await page.waitForFunction(() => !document.querySelector('[data-card-hover-preview][data-visible="true"]'));
  assert.deepEqual(errors, []);
 } finally { await browser.close(); await vite.close(); }
});


test('the stack starts past the mana rail and looked-at cards open from the Deck chip', {timeout:60000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch({args:['--disable-webgl']});
 try {
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  await page.route('https://**/*',r=>r.fulfill({status:404,body:''}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?look&targetLook`,{waitUntil:'domcontentloaded'});
  const deck=page.locator('[data-my-zone] [data-zone-pile="library"][data-looking="true"]');
  await deck.waitFor();
  await page.waitForTimeout(1200);
  const geometry=await page.evaluate(()=>{
   const board=document.querySelector('[data-my-zone] .my-zone-board-shell');
   const rect=el=>el?.getBoundingClientRect().toJSON();
   const mana=[...document.querySelectorAll('[data-my-zone] .my-zone-mana-rail, .fc-local-mana-rail, .local-player-mana-dock')]
     .map(el=>el.getBoundingClientRect()).filter(r=>r.width>0&&r.width<160);
   return {rail:rect(board.querySelector('.my-zone-stack-rail')),manaRight:Math.max(0,...mana.map(r=>r.right)),identity:rect(document.querySelector('.table-shared-player-header')),pile:rect(document.querySelector('.card-stack-slot[data-top="true"] .stack-card--frame')),pager:rect(document.querySelector('.card-stack-pager')),automaticFrames:document.querySelectorAll('[data-automatic-stack-preview="true"]').length,top:board.style.getPropertyValue('--stack-area-top')};
  });
  assert.ok(geometry.rail.left>=geometry.manaRight,`stack clears the mana rail: ${JSON.stringify(geometry)}`);
  const firstColumn=await page.locator('[data-my-zone] .battlefield-row .game-card').evaluateAll(cards=>Math.min(...cards.map(card=>card.getBoundingClientRect().left)));
  assert.ok(geometry.rail.right<=firstColumn-6,'stack stops before the first field column');
  assert.equal(geometry.top,'12px');
  assert.equal(geometry.automaticFrames,0,'no duplicate stack frame');
  assert.ok(geometry.pager.bottom<=geometry.identity.top,'pile and pager clear the player header');
  // Holding legal picks, the Deck chip opens to the left with its looked-at cards.
  await deck.hover();
  const menu=page.locator('.zone-pile-menu--icon[data-local-zone-strip="true"]');
  await menu.waitFor();
  const menuBox=await menu.boundingBox(), deckBox=await deck.boundingBox();
  assert.ok(menuBox.x>=0 && menuBox.y>=0,'strip stays on screen');
  assert.ok(menuBox.x+menuBox.width<=deckBox.x+2,'strip extends left of the chip');
  assert.deepEqual(await menu.locator('[data-zone-card="library"]').evaluateAll(cards=>cards.map(card=>card.dataset.objectId)),['7002','7001']);
 } finally { await browser.close(); await vite.close(); }
});
