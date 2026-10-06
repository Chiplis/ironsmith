import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('arrow-key hand navigation keeps the selected card above the phase tracker', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const width of [1365, 2048]) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      await page.route('https://**/*', route => route.abort());
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`);
      await page.locator('.hand-card[data-object-id="4"]').focus();
      for (const key of ['ArrowRight', 'ArrowRight', 'ArrowLeft']) {
        await page.keyboard.press(key);
        await page.waitForTimeout(350);
        const aboveTracker = await page.evaluate(() => {
          const card = document.activeElement;
          if (!card?.classList.contains('keyboard-selected') || card.classList.contains('hovered')) return false;
          const rect = card.getBoundingClientRect();
          const phase = document.querySelector('.topbar-shell').getBoundingClientRect();
          const y = Math.max(rect.top, phase.top) + 12;
          return y < Math.min(rect.bottom, phase.bottom)
            && card.contains(document.elementFromPoint(rect.left + rect.width / 2, y));
        });
        assert.ok(aboveTracker, `keyboard-selected card stays above phase controls at ${width}px after ${key}`);
      }
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});

test('large desktop hands keep compressing without horizontal scrolling', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const width of [1365, 2048]) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      await page.route('https://**/*', route => route.abort());
      let previousGap = Infinity;
      let cardWidth;
      for (const count of [10, 18, 40]) {
        await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html?count=${count}`);
        await page.locator('.hand-layout-item').last().waitFor();
        const geometry = await page.evaluate(() => {
          const scroll = document.querySelector('.hand-zone-scroll');
          const slots = Array.from(document.querySelectorAll('.hand-layout-item'), el => el.getBoundingClientRect());
          const viewport = scroll.closest('.hand-zone-surface').getBoundingClientRect();
          return { count: slots.length, gap: slots[1].left - slots[0].left, cardWidth: slots[0].width,
            left: slots[0].left, right: slots.at(-1).right, viewportLeft: viewport.left, viewportRight: viewport.right,
            overflowX: getComputedStyle(scroll).overflowX };
        });
        assert.equal(geometry.count, count);
        assert.ok(geometry.gap < previousGap, 'overlap increases as cards are added');
        previousGap = geometry.gap;
        cardWidth ??= geometry.cardWidth;
        assert.equal(geometry.cardWidth, cardWidth, 'cards retain their resting size');
        assert.ok(geometry.left >= geometry.viewportLeft - 5 && geometry.right <= geometry.viewportRight + 5, `all ${count} slots fit at ${width}px: ${JSON.stringify(geometry)}`);
        assert.equal(geometry.overflowX, 'visible');
        await page.locator('.hand-zone-scroll').dispatchEvent('wheel', { deltaX: 200, deltaY: 0 });
        assert.equal(await page.locator('.hand-zone-scroll').evaluate(el => el.scrollLeft), 0);
      }
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});

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
    const card = page.locator('.game-card.hand-card').nth(3);
    await card.waitFor();
    await page.mouse.click(20, 20);
    const resting = await card.boundingBox();
    await page.mouse.move(resting.x + resting.width / 2, resting.y + 24);
    await page.waitForFunction(() => document.querySelector('.hand-card[data-object-id="4"]')?.classList.contains('hovered'), null, { timeout: 3000 });
    await page.waitForFunction(() => {
      const card = document.querySelector('.hand-card[data-object-id="4"]');
      return card && card.getBoundingClientRect().height > 400;
    }, null, { timeout: 3000 });
    assert.ok((await card.boundingBox()).width > resting.width * 2, 'the hovered card fans out again');

    await page.mouse.click(20, 20);
    await page.waitForFunction(() => !document.querySelector('.hand-card.hovered'));
    await card.dispatchEvent('pointermove', { pointerType: 'touch', clientX: resting.x + 20, clientY: resting.y + 24 });
    assert.equal(await page.locator('.hand-card.hovered').count(), 0, 'touch movement does not undo outside-tap dismissal');
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
    const card = page.locator('.game-card.hand-card').nth(3);
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
    const card = page.locator('.game-card.hand-card').nth(3);
    await card.waitFor();
    await page.waitForTimeout(400);

    const resting = await card.boundingBox();
    // Hover by the card's top edge: its centre is the part of the fan that hangs
    // below the window, so that is where a player actually reaches for it.
    const grabbed = { x: resting.x + (resting.width / 2), y: resting.y + 24 };
    await page.mouse.move(grabbed.x, grabbed.y);
    await page.waitForTimeout(600);
    const hovered = await card.boundingBox();

    await page.mouse.move(20, 20);
    await page.waitForTimeout(200);
    await page.locator('.game-card[data-object-id="100"]').hover();
    const preview = page.locator('.floating-card-preview[data-visible="true"]');
    await preview.waitFor();
    await page.waitForTimeout(300);
    const fieldPreview = await preview.boundingBox();
    assert.ok(Math.abs(hovered.height - fieldPreview.height) < 2, `hand height ${hovered.height} matches field height ${fieldPreview.height}`);
    assert.ok(Math.abs(hovered.width - fieldPreview.width) < 3, `hand width ${hovered.width} matches field width ${fieldPreview.width}`);
    assert.ok(hovered.width > resting.width * 2, `hovered width ${hovered.width} vs resting ${resting.width}`);

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


test('highlighted hand neighbors smoothly spread away from the hovered card', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.route('https://**/*', route => route.abort());
    await page.addInitScript(() => {
      window.__handDecision = { kind: 'priority', player: 0, actions: Array.from({ length: 7 }, (_, i) => ({
        kind: 'cast_spell', object_id: i + 1, index: i, label: 'Cast Myr Moonvessel',
        action_ref: { kind: 'cast_spell', object_id: i + 1 },
      })) };
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`);
    const neighbor = page.locator('.hand-card[data-object-id="3"]');
    await neighbor.waitFor();
    await page.waitForTimeout(400);
    assert.equal(await neighbor.evaluate(node => node.classList.contains('card-action-available')), true);
    const restingX = await neighbor.evaluate(node => node.getBoundingClientRect().x);
    const resting = await page.locator('.hand-card[data-object-id="4"]').boundingBox();
    await page.mouse.move(resting.x + resting.width / 2, resting.y + 24);
    await page.waitForFunction(() => document.querySelector('.hand-card[data-object-id="4"]')?.classList.contains('hovered'));
    const motion = await neighbor.evaluate(node => {
      const animation = node.getAnimations().find(animation => animation.transitionProperty === 'transform');
      return animation ? { duration: animation.effect.getTiming().duration, state: animation.playState } : null;
    });
    assert.ok(motion, 'the highlighted neighbor has a live transform transition');
    assert.equal(motion.state, 'running');
    assert.ok(motion.duration >= 220, 'spreading remains smooth');
    await page.waitForTimeout(450);
    const spreadX = await neighbor.evaluate(node => node.getBoundingClientRect().x);
    assert.ok(spreadX < restingX - 10, 'the left neighbor moves away from the hovered card');
  } finally { await browser.close(); await vite.close(); }
});
