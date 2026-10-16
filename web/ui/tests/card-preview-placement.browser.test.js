import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('live preview relocates around controls and restores its position when they close', { timeout: 30000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://**', route => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-preview-source.html`);
    await page.evaluate(() => {
      // Model the local half starting below the central phase tracker.
      const localZone = document.createElement('div');
      localZone.dataset.myZone = 'true';
      localZone.innerHTML = '<div class="my-zone-board-shell" style="position:fixed;top:600px;left:0;width:1440px;height:300px;pointer-events:none"></div>';
      document.body.append(localZone);
      const phase = document.createElement('div');
      phase.className = 'phase-track-group';
      phase.textContent = 'Phase tracker';
      Object.assign(phase.style, { position: 'fixed', left: '0', top: '420px', width: '1440px', height: '60px', zIndex: '160' });
      document.body.append(phase);
      document.querySelectorAll('.battlefield-row-card').forEach(el => { el.style.marginTop = '560px'; });
    });
    await page.getByAltText('Field card 1').hover();
    const preview = page.locator('.floating-card-preview[data-visible="true"]');
    await preview.waitFor();
    const initial = await preview.boundingBox();
    assert.ok(initial.y < 600, 'local card preview can extend into the opponent zone');
    assert.ok(initial.y < 480 && initial.y + initial.height > 420, 'frame can overlap the phase tracker');
    assert.ok(await page.evaluate(initial => {
      const hit = document.elementFromPoint(initial.x + initial.width / 2, 450);
      return !!hit?.closest('.floating-card-preview');
    }, initial), 'frame paints above the phase tracker');
    for (const className of ['decision-main-button', 'mana-payment-editor']) {
      const before = await preview.boundingBox();
      await page.evaluate(({ className, before }) => {
        const control = document.createElement('button');
        control.className = className;
        control.dataset.testObstacle = 'true';
        control.textContent = className;
        Object.assign(control.style, { position: 'fixed', left: `${before.x}px`, top: `${before.y + 150}px`,
          width: `${before.width}px`, height: '100px', zIndex: '1' });
        document.body.append(control);
      }, { className, before });
      await page.waitForFunction(() => {
        const frame = document.querySelector('.floating-card-preview[data-visible="true"]');
        const r = frame.getBoundingClientRect();
        return [...document.querySelectorAll('[data-test-obstacle]')].every(el => {
          const o = el.getBoundingClientRect();
          return r.right <= o.left || r.left >= o.right || r.bottom <= o.top || r.top >= o.bottom;
        });
      });
      const after = await preview.boundingBox();
      assert.ok(Math.abs(after.height - initial.height) < 1, 'keeps the full-size frame');
      assert.equal(await preview.evaluate(el => getComputedStyle(el).clipPath), 'none');
    }
    await page.locator('[data-test-obstacle]').evaluateAll(els => els.forEach(el => el.remove()));
    await page.waitForFunction(initial => {
      const r = document.querySelector('.floating-card-preview[data-visible="true"]').getBoundingClientRect();
      return Math.abs(r.left - initial.x) < 1 && Math.abs(r.top - initial.y) < 1;
    }, initial);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
