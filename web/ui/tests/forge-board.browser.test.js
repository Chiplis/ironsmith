import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('forge follows crowded zones, preserves input, resizes, and releases GPU resources', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch({ args: ['--enable-unsafe-swiftshader'] });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    await page.addInitScript(() => {
      window.forgeDraws = 0;
      const draw = WebGL2RenderingContext.prototype.drawElements;
      WebGL2RenderingContext.prototype.drawElements = function(...args) { window.forgeDraws++; return draw.apply(this, args); };
    });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/forge-board.html`);
    await page.waitForSelector('.forge-board[data-renderer="webgl"] canvas');
    await page.waitForFunction(() => Number(document.querySelector('.forge-board').dataset.zoneCount) === 3);
    await page.waitForSelector('.forge-board[data-models="6"][data-terrain="ready"]');
    await page.locator('.battlefield-row button').first().click();
    assert.equal(await page.locator('output').textContent(), '1');
    const scenarios = [
      { counts: [0, 0], hand: 0 }, { counts: [0, 1], hand: 6 },
      { counts: [12, 100], hand: 20 }, { counts: [20, 30], hand: 7, attachments: true },
      { counts: [0, 0], hand: 7 }, { counts: [20, 30], hand: 7 },
      { counts: [35, 15], hand: 7, combat: true },
      { counts: [1, 30, 4, 12], hand: 7, turn: 2, piles: true },
      { counts: [3, 7], hand: 40, viewer: true }, { counts: [3, 20], hand: 7, locked: true },
    ];
    for (const scenario of scenarios) {
      await page.evaluate(s => window.forgeScenario(s), scenario);
      await page.waitForFunction(s => document.querySelectorAll('.battlefield-row').length === s.counts.length
        && document.querySelectorAll('.battlefield-row > button').length === s.counts.reduce((a, b) => a + b, 0), scenario);
      await page.waitForTimeout(150);
      const measurements = await page.evaluate(() => window.forgeMeasure());
      assert.ok(measurements.zones.length >= scenario.counts.length);
      assert.deepEqual([...new Set(measurements.zones.filter(z => z.battlefield).map(z => z.owner))], scenario.counts.map((_, i) => String(i)));
      if (scenario.piles) {
        for (let owner = 0; owner < scenario.counts.length; owner++) {
          for (const type of ['graveyard', 'exile']) assert.ok(measurements.zones.some(z => z.owner === String(owner) && z.type === type));
        }
      }
      for (const { rect } of measurements.zones) assert.ok(rect.left >= 0 && rect.top >= 0 && rect.right <= 1280 && rect.bottom <= 800);
      assert.equal(await page.locator('.forge-board').evaluate(el => getComputedStyle(el).pointerEvents), 'none');
    }
    await page.evaluate(() => window.forgeScenario({ counts: [3, 7], hand: 7, accents: { 0: '#33aaff', 1: '#bb55ee' } }));
    await page.waitForFunction(() => document.querySelector('.forge-board').dataset.playerLights === '0:#33aaff,1:#bb55ee');
    await page.waitForTimeout(1900);
    await page.screenshot({ path: '/tmp/ironsmith-forge-board.png' });
    await page.setViewportSize({ width: 760, height: 390 });
    await page.waitForTimeout(250);
    assert.deepEqual(await page.locator('canvas').evaluate(el => ({ w: el.clientWidth, h: el.clientHeight })), { w: 760, h: 390 });
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.waitForFunction(() => document.querySelector('.forge-board').dataset.motion === 'reduced');
    await page.waitForTimeout(2000);
    const idleDraws = await page.evaluate(() => window.forgeDraws);
    await page.waitForTimeout(200);
    assert.equal(await page.evaluate(() => window.forgeDraws), idleDraws, 'reduced motion must stop rendering when settled');
    await page.evaluate(() => {
      window.restoreForgeContext = document.querySelector('canvas').getContext('webgl2').getExtension('WEBGL_lose_context');
      window.restoreForgeContext.loseContext();
    });
    await page.waitForFunction(() => document.querySelector('.forge-board').dataset.renderer === 'fallback');
    await page.locator('.battlefield-row button').first().click();
    assert.equal(await page.locator('output').textContent(), '2');
    await page.waitForTimeout(200);
    await page.evaluate(() => window.restoreForgeContext.restoreContext());
    await page.waitForFunction(() => document.querySelector('.forge-board').dataset.renderer === 'webgl');
    await page.evaluate(() => window.forgeScenario({ counts: [3, 7], unmounted: true }));
    await page.waitForFunction(() => !document.querySelector('canvas'));
    await page.evaluate(() => window.forgeScenario({ counts: [3, 7] }));
    await page.waitForSelector('.forge-board[data-renderer="webgl"] canvas');
    assert.equal(await page.locator('canvas').count(), 1);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});

test('WebGL-unavailable fallback remains playable', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch({ args: ['--disable-webgl'] });
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/forge-board.html`);
    await page.waitForFunction(() => Number(document.querySelector('.forge-board')?.dataset.zoneCount) > 0);
    assert.equal(await page.locator('.forge-board').getAttribute('data-renderer'), 'fallback');
    await page.locator('.battlefield-row button').first().click();
    assert.equal(await page.locator('output').textContent(), '1');
  } finally { await browser.close(); await vite.close(); }
});

test('real WASM puzzle mounts the forge on desktop and mobile', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch({ args: ['--enable-unsafe-swiftshader'] });
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const puzzle = { version: 1, players: [
      { name: 'Forge', life: 20, zones: { battlefield: ['Forest', 'Island', 'Llanowar Elves'], hand: ['Forest', 'Grizzly Bears'], library: ['Forest', 'Forest'] } },
      { name: 'Opponent', life: 20, zones: { battlefield: ['Mountain', 'Mountain', 'Grizzly Bears'], library: ['Mountain'] } },
    ] };
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?puzzle=${Buffer.from(JSON.stringify(puzzle)).toString('base64url')}`);
    await page.waitForSelector('.forge-board[data-renderer="webgl"] canvas', { timeout: 90000 });
    await page.waitForFunction(() => Number(document.querySelector('.forge-board').dataset.zoneCount) > 0);
    assert.ok(await page.locator('.battlefield-row .game-card').count() >= 3);
    assert.equal(await page.locator('.table-shell').first().evaluate(el => getComputedStyle(el).backgroundColor), 'rgba(0, 0, 0, 0)');
    assert.ok(await page.locator('.battlefield-artwork-card').count() >= 3);
    assert.ok(await page.locator('.battlefield-small-land').count() >= 2);
    const creature = page.locator('.battlefield-row[data-bf-side="bottom"] .battlefield-artwork-card').filter({ has: page.locator('[data-kind="creature"]') }).first();
    const land = page.locator('.battlefield-small-land').first();
    assert.ok((await land.boundingBox()).height < (await creature.boundingBox()).height * .8);
    const creatureBounds = await creature.boundingBox();
    assert.ok(Math.abs(creatureBounds.width / creatureBounds.height - 1.15) < .025, 'compact creatures use the squarer tile ratio');
    const landBounds = await land.boundingBox();
    assert.ok(landBounds.width / landBounds.height > 1 && landBounds.width / landBounds.height < 1.25, 'compact lands should use a short landscape tile');
    const ownLands = page.locator('.battlefield-row[data-bf-side="bottom"] .battlefield-small-land');
    const firstLand = await ownLands.nth(0).boundingBox(), nextLand = await ownLands.nth(1).boundingBox();
    assert.ok(Math.abs(nextLand.x - firstLand.x - firstLand.width - 6) < 2, 'lands should pack with a six-pixel gap');
    assert.equal(await creature.locator('.arena-permanent-name').innerText(), 'Llanowar Elves');
    await page.locator('.topbar-menu-trigger:visible').first().click();
    await page.getByLabel('Battlefield cards', { exact: true }).selectOption('full');
    await page.getByLabel('Noncreature lands', { exact: true }).selectOption('standard');
    assert.equal(await page.locator('.battlefield-artwork-card').count(), 0);
    assert.ok(await page.locator('.battlefield-portrait-card').count() >= 3);
    await page.getByLabel('Noncreature lands', { exact: true }).selectOption('compact');
    assert.ok(await page.locator('.battlefield-small-land').count() >= 2);
    assert.ok(await page.locator('.battlefield-portrait-card').count() >= 1);
    await page.getByLabel('Battlefield cards', { exact: true }).selectOption('compact');
    await page.keyboard.press('Escape');
    await page.reload();
    await page.waitForSelector('.battlefield-artwork-card', { timeout: 90000 });
    assert.deepEqual(await page.evaluate(() => JSON.parse(localStorage.getItem('ironsmith.battlefieldAppearance'))), { compactCards: true, compactLands: true });
    const assertOpponentRowOrder = async () => {
      const opponentCreature = page.locator('.battlefield-row[data-bf-side="top"] .arena-permanent[data-kind="creature"]').first();
      const opponentLand = page.locator('.battlefield-row[data-bf-side="top"] .arena-permanent[data-kind="land"]').first();
      await opponentCreature.waitFor({ state: 'visible' });
      const front = await opponentCreature.boundingBox(), back = await opponentLand.boundingBox();
      assert.ok(front.y >= back.y + back.height, 'opponent creatures must face our creatures, with their resources behind');
    };
    await assertOpponentRowOrder();
    await page.setViewportSize({ width: 844, height: 390 });
    await page.waitForSelector('.mobile-mtga-battlefield-band');
    await page.waitForTimeout(300);
    await assertOpponentRowOrder();
    assert.equal(await page.locator('.forge-board canvas').count(), 1);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
