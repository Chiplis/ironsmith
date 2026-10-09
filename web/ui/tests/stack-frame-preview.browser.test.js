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
      const entry=document.querySelector('.my-zone-stack-rail .stack-timeline-entry').getBoundingClientRect();
      return frame && Math.abs(frame.left-12)<2 && Math.abs(frame.top-entry.bottom-4)<2 && frame.width<=301;
    });
    const bounds=await frame.boundingBox();
    assert.ok(bounds.height>300,'rail frame remains readable');
    assert.ok(bounds.y+bounds.height<=viewport.height-7,'frame fits screen');
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


test('Look stays above a fixed stack without consuming frame height', {timeout:60000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch({args:['--disable-webgl']});
 try {
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  await page.route('https://**/*',r=>r.fulfill({status:404,body:''}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?look&targetLook`,{waitUntil:'domcontentloaded'});
  await page.locator('[data-zone-pile="look"]').waitFor();
  await page.waitForTimeout(1200);
  const geometry=await page.evaluate(()=>{
   const board=document.querySelector('[data-my-zone] .my-zone-board-shell');
   const rect=el=>el.getBoundingClientRect().toJSON();
   return {opponent:rect(document.querySelector('.battlefield-panel--opponents')),board:rect(board),rail:rect(board.querySelector('.my-zone-stack-rail')),look:rect(board.querySelector('.player-look-pile')),mana:rect(document.querySelector('.local-player-mana-dock')),identity:rect(document.querySelector('.table-shared-player-header')),preview:rect(document.querySelector('[data-automatic-stack-preview="true"]')),top:board.style.getPropertyValue('--stack-area-top')};
  });
  await page.screenshot({path:'/tmp/stack-look-layout.png'});
  assert.ok(geometry.rail.top>=geometry.look.bottom+6,'stack clears Look');
  assert.ok(geometry.look.left<16,'Look sits on the far left');
  assert.ok(Math.abs(geometry.rail.left)<1,'stack begins at screen edge');
  const firstColumn=await page.locator('[data-my-zone] .battlefield-row .game-card').evaluateAll(cards=>Math.min(...cards.map(card=>card.getBoundingClientRect().left)));
  assert.ok(Math.abs(firstColumn-geometry.rail.right-8)<2,'stack stops before the first field column');
  assert.equal(geometry.top,'12px','Look never moves the stack anchor');
  const lookSlot=await page.locator('.player-look-pile .zone-pile-slot').boundingBox();
  assert.ok(Math.abs(geometry.rail.top-lookSlot.y-lookSlot.height-8)<2,'enlarged Look remains above the stack');
  assert.ok(geometry.mana.top>=geometry.identity.bottom,'mana sits beneath identity');
  assert.ok(geometry.mana.bottom<=900 && geometry.mana.bottom>=875,'mana sits at the screen bottom');
  assert.ok(geometry.preview.bottom<=geometry.mana.top-6,'frame clears mana');
  assert.ok(geometry.preview.left<=geometry.identity.left && geometry.preview.right>=geometry.identity.right && geometry.preview.top<=geometry.identity.top && geometry.preview.bottom>=geometry.identity.bottom,'frame fully covers identity');
  assert.ok(geometry.rail.width>=300,'stack gains a battlefield column');
  const lookTile=await page.locator('[data-zone-pile="look"]').boundingBox();
  assert.ok(lookTile.x>=0,'enlarged Look stays inside the left screen edge');
  await page.locator('[data-zone-pile="look"]').hover();
  await page.locator('.zone-pile-menu--look').waitFor();
  const menu=await page.locator('.zone-pile-menu--look').boundingBox();
  assert.ok(menu.x>=0 && menu.y>=0,'expanded Look stays on screen');
  await page.locator('[data-zone-card="look"]').first().hover();
  const lookInspector=page.locator('[data-look-stack-preview="true"][data-visible="true"]');
  await lookInspector.waitFor();
  assert.equal(await lookInspector.getAttribute('data-preview-object-id'),await page.locator('[data-zone-card="look"]').first().getAttribute('data-object-id'));
  const lookInspectorBounds=await lookInspector.boundingBox();
  assert.ok(Math.abs(lookInspectorBounds.x-geometry.rail.left)<2,'Look inspector uses the stack rail');
  assert.ok(lookInspectorBounds.x+lookInspectorBounds.width<=geometry.rail.right+1,'Look inspector stays in stack width');
  await page.waitForTimeout(300);
  const hoveredLook=await page.locator('[data-zone-card="look"]').first().boundingBox();
  const hoveredMenu=await page.locator('.zone-pile-menu--look').boundingBox();
  assert.ok(hoveredLook.x>=0 && hoveredLook.y>=0,'hover-enlarged Look card stays on screen');
  assert.ok(hoveredMenu.x>=0 && hoveredMenu.x+hoveredMenu.width<=1440,'hovered Look strip stays within the viewport');
  await page.mouse.move(1400,20);
  await page.locator('[data-automatic-stack-preview="true"][data-visible="true"]').waitFor();
  await page.locator('.player-look-pile').evaluate(el=>el.style.display='none');
  await page.waitForTimeout(400);
  const after=await page.locator('[data-automatic-stack-preview="true"]').boundingBox();
  assert.ok(Math.abs(after.height-geometry.preview.height)<2,'hiding Look does not change preview height');
  assert.ok(Math.abs(after.y-geometry.preview.top)<2,'hiding Look does not move the frame');
  await page.screenshot({path:'/tmp/stack-look-layout.png'});
 } finally {await browser.close();await vite.close();}
});
