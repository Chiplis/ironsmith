import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { ANIMATION_ZONES, ZONE_COLORS } from '../src/lib/game-effect-events.js';

test('zone flights render, clean up, respect reduced motion, and keyword shields are green', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-flight-matrix.html`;
    await page.goto(url);
    await page.getByRole('button', { name: 'Grant hexproof' }).click();
    await page.locator('.game-fx-semantic').waitFor();
    assert.equal(await page.locator('.game-fx-semantic').evaluate(el => getComputedStyle(el).color), 'rgb(88, 233, 154)');
    const events = ANIMATION_ZONES.map((zone, index) => ({ type: 'zone-flight', id: `route-${index}`, playerKey: '0', fromZone: 'library', toZone: zone, color: ZONE_COLORS[zone], cardName: '' }));
    await page.evaluate(events => window.dispatchEvent(new CustomEvent('ironsmith:game-effect-animations-debug', { detail: { events } })), events);
    await page.waitForTimeout(450);
    assert.equal(await page.locator('.game-fx-flight').count(), ANIMATION_ZONES.length);
    assert.equal(await page.locator('.game-fx-flight-front').count(), 0);
    await page.screenshot({ path: '/tmp/zone-flight-palette.png' });
    await page.waitForTimeout(1800);
    assert.equal(await page.locator('.game-fx-flight, .game-fx-semantic').count(), 0);
    // Exercise real snapshot collection, not the debug event entry point.
    await page.getByRole('button', { name: 'Hidden draw' }).click();
    await page.locator('.game-fx-flight').waitFor();
    assert.equal(await page.locator('.game-fx-flight').count(), 1);
    assert.equal(await page.locator('.game-fx-flight-front').count(), 0);
    await page.waitForTimeout(1400);
    assert.equal(await page.locator('.game-fx-flight').count(), 0);
    assert.deepEqual(errors, []);
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.reload();
    await page.getByRole('button', { name: 'Grant hexproof' }).click();
    assert.equal(await page.locator('.game-fx-semantic').count(), 0);
  } finally { await browser.close(); await vite.close(); }
});
