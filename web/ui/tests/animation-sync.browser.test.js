import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('draw landing signals completion and ETB resolution never hides its source', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-flight-matrix.html`);
    await page.getByRole('button', { name: 'Grant hexproof' }).click();
    await page.locator('.game-fx-semantic').waitFor();
    await page.evaluate(() => {
      window.landings = [];
      window.addEventListener('ironsmith:hand-flight-landed', event => window.landings.push(event.detail.objectId));
      window.dispatchEvent(new CustomEvent('ironsmith:game-effect-animations-debug', { detail: { events: [
        { type: 'zone-flight', id: 'draw-sync', playerKey: '0', fromZone: 'library', toZone: 'hand', objectId: 'sync-card', cardName: '' },
      ] } }));
    });
    await page.locator('.game-fx-flight').waitFor();
    assert.deepEqual(await page.evaluate(() => window.landings), []);
    await page.waitForFunction(() => window.landings.includes('sync-card'));
    const hidden = await page.evaluate(async () => {
      const { playExit } = await import('/src/components/right-rail/card-stack-motion.js');
      const source = document.createElement('div');
      source.className = 'game-card';
      source.dataset.objectId = 'new-permanent';
      source.style.cssText = 'width:100px;height:140px';
      const ability = document.createElement('div');
      ability.style.cssText = 'width:100px;height:140px';
      document.body.append(source, ability);
      playExit({ node: ability, entry: { stable_id: 'source', ability_kind: 'triggered' },
        state: { zone_transitions: [{ stable_id: 'source', from_zone: 'stack', to_zone: 'battlefield', new_object_id: 'new-permanent' }] },
        inspectId: 'new-permanent', owner: 0, rgb: '255, 255, 255', onMotion: () => {} });
      return getComputedStyle(source).visibility;
    });
    assert.equal(hidden, 'visible');
  } finally { await browser.close(); await vite.close(); }
});
