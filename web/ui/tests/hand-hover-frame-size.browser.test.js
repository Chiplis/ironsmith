import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('pre-game hand hover resumes after clicking outside the fan', { timeout: 30000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.addInitScript(() => {
      window.__handDecision = { kind: 'priority', player: 0, actions: [
        { index: 0, kind: 'keep_hand', label: 'Keep hand' },
        { index: 1, kind: 'take_mulligan', label: 'Mulligan' },
      ] };
    });
    await page.route('https://**/*', route => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`);
    const card = page.locator('.game-card.hand-card[data-object-id="4"]');
    await card.waitFor();
    await page.mouse.click(20, 20);
    const resting = await card.boundingBox();
    await page.mouse.move(resting.x + resting.width / 2, resting.y + 24);
    await page.waitForFunction(() => document.querySelector('.hand-card[data-object-id="4"]')?.classList.contains('hovered'), null, { timeout: 3000 });
    await page.waitForFunction(() => {
      const card = document.querySelector('.hand-card[data-object-id="4"]');
      return card && card.closest('.hand-hover-portal') && card.getBoundingClientRect().height > 400;
    }, null, { timeout: 3000 });
    assert.ok((await card.boundingBox()).width > resting.width * 2, 'the hovered card fans out again');

    await page.mouse.click(20, 20);
    await page.waitForFunction(() => !document.querySelector('.hand-card.hovered'));
    await card.dispatchEvent('pointermove', { pointerType: 'touch', clientX: resting.x + 20, clientY: resting.y + 24 });
    assert.equal(await page.locator('.hand-card.hovered').count(), 0, 'touch movement does not undo outside-tap dismissal');
    await card.focus();
    await page.keyboard.press('ArrowRight');
    assert.equal(await page.evaluate(() => document.activeElement?.dataset.objectId), '5');
    await page.keyboard.press('ArrowLeft');
    assert.equal(await page.evaluate(() => document.activeElement?.dataset.objectId), '4', 'keyboard focus stays within the hand across portaled cards');
    assert.ok(await page.locator('.hand-hover-portal').evaluate(el => {
      const card = el.getBoundingClientRect();
      const phase = document.querySelector('.topbar-shell').getBoundingClientRect();
      const overlapY = Math.max(card.top, phase.top) + 12;
      return overlapY < Math.min(card.bottom, phase.bottom)
        && el.contains(document.elementFromPoint(card.left + card.width / 2, overlapY));
    }), 'the keyboard-selected card renders above the phase tracker where they overlap');
  } finally { await browser.close(); await vite.close(); }
});

test('overflowing hands keep edge previews visible without moving the scroll position', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const viewport of [{ width: 2048, height: 764 }, { width: 1365, height: 768 }]) {
      const page = await browser.newPage({ viewport });
      await page.route('https://**/*', route => route.abort());
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html?count=18`);
      const scroll = page.locator('.hand-zone-scroll[data-hand-overflow="true"]');
      await scroll.waitFor();
      for (const objectId of [1, 18]) {
        await page.mouse.click(20, 20);
        await scroll.evaluate((el, id) => { el.scrollLeft = id === 1 ? 0 : el.scrollWidth; }, objectId);
        await page.waitForTimeout(300);
        const before = await scroll.evaluate(el => el.scrollLeft);
        const card = page.locator(`.hand-card[data-object-id="${objectId}"]`);
        const resting = await card.boundingBox();
        await page.mouse.move(resting.x + resting.width * (objectId === 1 ? .4 : .6), resting.y + 60);
        const portal = page.locator('.hand-hover-portal');
        await portal.waitFor({ timeout: 5000 });
        await page.waitForTimeout(200);
        const enlarged = await portal.boundingBox();
        assert.ok(enlarged.x >= 8 && enlarged.x + enlarged.width <= viewport.width - 7);
        assert.ok(enlarged.y >= viewport.height * .45 && enlarged.y + enlarged.height <= viewport.height);
        assert.equal(await scroll.evaluate(el => el.scrollLeft), before);
        const cornersVisible = await portal.evaluate(el => {
          const r = el.getBoundingClientRect();
          return [[r.left + 12, r.top + 12], [r.right - 12, r.bottom - 12]].every(([x, y]) => el.contains(document.elementFromPoint(x, y)));
        });
        assert.ok(cornersVisible, `card ${objectId} is fully accessible at ${viewport.width}px`);
      }
      assert.ok((await scroll.boundingBox()).height <= 220, 'the scroll container does not reserve preview headroom');
      await page.screenshot({ path: `/tmp/ironsmith-hand-preview-${viewport.width}.png` });
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});

test('hovered hand cards show live text before their image URL and art arrive', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  let releaseLookup, releaseArt;
  const lookupGate = new Promise(resolve => { releaseLookup = resolve; });
  const artGate = new Promise(resolve => { releaseArt = resolve; });
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/cards/myr-moonvessel.json', async route => {
      await lookupGate;
      await route.continue();
    });
    await page.route('https://api.scryfall.com/**', route => route.fulfill({ status: 404, body: '' }));
    await page.route('https://cards.scryfall.io/**', async route => {
      await artGate;
      await route.fulfill({ contentType: 'image/svg+xml', headers: { 'access-control-allow-origin': '*' }, body: '<svg xmlns="http://www.w3.org/2000/svg" width="488" height="684"><rect width="488" height="684" fill="#917659"/></svg>' });
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`, { waitUntil: 'domcontentloaded' });
    const card = page.locator('.game-card.hand-card[data-object-id="4"]');
    await card.waitFor();
    const box = await card.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + 24);
    const frame = card.locator('[data-loading-frame="true"]');
    await frame.waitFor();
    assert.match(await frame.innerText(), /Myr Moonvessel/);
    assert.match(await frame.innerText(), /When this creature dies/);
    releaseLookup();
    await page.waitForFunction(() => Boolean(document.querySelector('.hand-card.hovered')?.dataset.cardImageUrl));
    assert.equal(await frame.isVisible(), true, 'frame remains while the actual image downloads');
    releaseArt();
    await page.waitForFunction(() => !document.querySelector('.hand-card.hovered .battlefield-prepared-frame'));
    assert.deepEqual(errors, []);
  } finally { releaseLookup(); releaseArt(); await browser.close(); await vite.close(); }
});

test('a hovered hand card matches the local battlefield preview size', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.route('https://api.scryfall.com/**', route => route.fulfill({ status: 404, body: '' }));
    await page.route('https://cards.scryfall.io/**', route => route.fulfill({ contentType: 'image/svg+xml', headers: { 'access-control-allow-origin': '*' }, body: '<svg xmlns="http://www.w3.org/2000/svg" width="488" height="684"><rect width="488" height="684" fill="#917659"/></svg>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`);
    const card = page.locator('.game-card.hand-card[data-object-id="4"]');
    await card.waitFor();
    await page.waitForTimeout(400);

    const resting = await card.boundingBox();
    // Hover by the card's top edge: its centre is the part of the fan that hangs
    // below the window, so that is where a player actually reaches for it.
    const grabbed = { x: resting.x + (resting.width / 2), y: resting.y + 24 };
    await page.mouse.move(grabbed.x, grabbed.y);
    await page.waitForTimeout(600);
    const hovered = await card.boundingBox();

    const field = page.locator('.game-card[data-object-id="100"]');
    await page.mouse.move(20, 20);
    await page.waitForTimeout(200);
    await field.hover();
    const preview = page.locator('.floating-card-preview[data-visible="true"]');
    await preview.waitFor();
    await page.waitForTimeout(300);
    const fieldPreview = await preview.boundingBox();
    assert.ok(Math.abs(hovered.height - fieldPreview.height) < 2, `hand height ${hovered.height} matches battlefield preview ${fieldPreview.height}`);
    assert.ok(Math.abs(hovered.width - fieldPreview.width) < 3, `hand width ${hovered.width} matches battlefield preview ${fieldPreview.width}`);

    // Hand cards scale from their bottom edge, so the zoom grows upwards on its
    // own and the lift stays small. It has to: if the enlarged card slid out
    // from under the pointer that opened it, the hover would drop, the card
    // would shrink back under the pointer and the two would oscillate — and a
    // drag started from that pointer would never reach the card at all.
    assert.ok(
      grabbed.x >= hovered.x && grabbed.x <= hovered.x + hovered.width
      && grabbed.y >= hovered.y && grabbed.y <= hovered.y + hovered.height,
      `the grabbed point ${JSON.stringify(grabbed)} stays on ${JSON.stringify(hovered)}`,
    );

    // The enlarged card still has to fit the window it grew into.
    assert.ok(hovered.y >= 0, `hovered top ${hovered.y} is on screen`);
  } finally {
    await browser.close();
    await vite.close();
  }
});

test('an enlarged hand card still starts a pointer drag', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.route('https://**/*', route => route.abort());
    await page.addInitScript(() => {
      window.__handDecision = { kind: 'priority', player: 0, actions: [
        { index: 0, kind: 'cast_spell', object_id: 4, label: 'Cast Myr Moonvessel', action_ref: { kind: 'cast_spell', spell_id: 4 } },
      ] };
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`);
    const card = page.locator('.hand-card[data-object-id="4"]');
    const resting = await card.boundingBox();
    const grip = { x: resting.x + resting.width / 2, y: resting.y + 24 };
    await page.mouse.move(grip.x, grip.y);
    await page.locator('.hand-hover-portal').waitFor();
    await page.mouse.down();
    await page.mouse.move(grip.x, grip.y - 100, { steps: 5 });
    await page.waitForFunction(() => String(window.__handDrag?.objectId) === '4');
    await page.mouse.move(grip.x + 100, grip.y - 140, { steps: 5 });
    assert.equal(await page.evaluate(() => window.__handDrag?.currentX), grip.x + 100);
    await page.mouse.up();
  } finally { await browser.close(); await vite.close(); }
});
