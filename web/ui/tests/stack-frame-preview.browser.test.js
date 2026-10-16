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


for (const commander of [false, true]) test(`Look follows ${commander ? 'Command' : 'Exile'} without consuming stack frame height`, {timeout:60000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch({args:['--disable-webgl']});
 try {
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  await page.route('https://**/*',r=>r.fulfill({status:404,body:''}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?look&targetLook${commander ? "&commander" : ""}`,{waitUntil:'domcontentloaded'});
  await page.locator('[data-zone-pile="look"]').waitFor();
  await page.waitForTimeout(1200);
  const geometry=await page.evaluate(()=>{
   const board=document.querySelector('[data-my-zone] .my-zone-board-shell');
   const rect=el=>el.getBoundingClientRect().toJSON();
   return {opponent:rect(document.querySelector('.battlefield-panel--opponents')),board:rect(board),rail:rect(board.querySelector('.my-zone-stack-rail')),look:rect(board.querySelector('.player-look-pile')),mana:rect(document.querySelector('.local-player-mana-dock')),identity:rect(document.querySelector('.table-shared-player-header')),pile:rect(document.querySelector('.card-stack-slot[data-top="true"] .stack-card--frame')),pager:rect(document.querySelector('.card-stack-pager')),automaticFrames:document.querySelectorAll('[data-automatic-stack-preview="true"]').length,top:board.style.getPropertyValue('--stack-area-top')};
  });
  await page.screenshot({path:'/tmp/stack-look-layout.png'});
  const exile = await page.locator(`[data-my-zone] [data-zone-pile=${commander ? 'command' : 'exile'}]`).boundingBox();
  assert.ok(geometry.look.top - exile.y - exile.height >= 5 && geometry.look.top - exile.y - exile.height <= 13, `Look immediately follows last zone: ${JSON.stringify({look:geometry.look, exile})}`);
  assert.ok(Math.abs(geometry.look.right - exile.x - exile.width) < 2, 'Look aligns with the right zone column');
  assert.ok(Math.abs(geometry.rail.left)<1,'stack begins at screen edge');
  const firstColumn=await page.locator('[data-my-zone] .battlefield-row .game-card').evaluateAll(cards=>Math.min(...cards.map(card=>card.getBoundingClientRect().left)));
  // The compact appearance's card pile narrows the old rail box to its cards.
  assert.ok(geometry.rail.right<=firstColumn-6,'stack stops before the first field column');
  assert.equal(geometry.top,'12px','Look never moves the stack anchor');
  const lookSlot=await page.locator('.player-look-pile .zone-pile-slot').boundingBox();
  assert.ok(lookSlot.y >= exile.y + exile.height + 5, 'enlarged Look stays below Exile');
  assert.ok(geometry.mana.top>=geometry.identity.bottom,'mana sits beneath identity');
  assert.ok(geometry.mana.bottom<=900 && geometry.mana.bottom>=875,'mana sits at the screen bottom');
  // The pile is the stack's frame: no second, automatic one opens.
  assert.equal(geometry.automaticFrames,0,'no duplicate stack frame');
  assert.ok(geometry.pager.bottom<=geometry.identity.top,'pile and pager clear the player header');
  assert.ok(Math.abs(geometry.pile.left-geometry.rail.left-12)<3,'front card sits in the stack box');
  // Allow for the cards' gentle float (up to 5px).
  assert.ok(geometry.pile.top>=0,'enlarged stack frame stays on screen');
  assert.ok(geometry.rail.width>=150,'stack holds a readable card');
  const lookTile=await page.locator('[data-zone-pile="look"]').boundingBox();
  assert.ok(lookTile.x>=0,'enlarged Look stays inside the left screen edge');
  await page.locator('[data-zone-pile="look"]').hover();
  await page.locator('.zone-pile-menu--look').waitFor();
  const menu=await page.locator('.zone-pile-menu--look').boundingBox();
  assert.ok(menu.x>=0 && menu.y>=0,'expanded Look stays on screen');
  assert.ok(menu.x < lookTile.x, 'Look extends left');
  await page.locator('[data-zone-card="look"]').first().hover();
  const lookInspector=page.locator('[data-look-stack-preview="true"][data-visible="true"]');
  await lookInspector.waitFor();
  assert.equal(await lookInspector.getAttribute('data-preview-object-id'),await page.locator('[data-zone-card="look"]').first().getAttribute('data-object-id'));
  const lookInspectorBounds=await lookInspector.boundingBox();
  assert.ok(Math.abs(lookInspectorBounds.x-geometry.rail.right-12)<2,'Look inspector uses the same beside-stack position');
  assert.ok(Math.abs(lookInspectorBounds.y-geometry.rail.top)<2,'Look inspector aligns with the stack top');
  await page.waitForTimeout(300);
  const hoveredLook=await page.locator('[data-zone-card="look"]').first().boundingBox();
  const hoveredMenu=await page.locator('.zone-pile-menu--look').boundingBox();
  assert.ok(hoveredLook.x>=0 && hoveredLook.y>=0,'hover-enlarged Look card stays on screen');
  assert.ok(hoveredMenu.x>=0 && hoveredMenu.x+hoveredMenu.width<=1440,'hovered Look strip stays within the viewport');
  await page.mouse.move(1400,20);
  await page.locator('.player-look-pile').evaluate(el=>el.style.display='none');
  await page.waitForTimeout(600);
  const after=await page.locator('.card-stack-slot[data-top="true"] .stack-card--frame').boundingBox();
  assert.ok(Math.abs(after.height-geometry.pile.height)<2,'hiding Look does not change the stack card size');
  // Allow for the float.
  assert.ok(Math.abs(after.y-geometry.pile.top)<7,'hiding Look does not move the stack');
  await page.screenshot({path:'/tmp/stack-look-layout.png'});
 } finally {await browser.close();await vite.close();}
});
