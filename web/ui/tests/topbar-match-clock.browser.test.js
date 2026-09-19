import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

// The per-player clocks ride with the turn status, beside the perspective
// picker: the mulligan prompt opens over the left of the toolbar, which is
// where they used to sit.
test('match clocks sit beside the perspective picker, inside the turn status', {timeout: 60000}, async () => {
  const vite = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1280, height: 300}});
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/topbar-match-clock.html`);
    await page.locator('.topbar-phase-clock').waitFor({timeout: 30000});
    const placement = await page.evaluate(() => {
      const clock = document.querySelector('.topbar-phase-clock');
      const status = document.querySelector('.topbar-phase-status');
      const decisionHost = document.querySelector('[data-topbar-main-decision-host]');
      const box = node => node?.getBoundingClientRect().toJSON() || null;
      const clockBox = box(clock), hostBox = box(decisionHost);
      return {
        insideStatus: clock?.closest('.topbar-phase-status') === status,
        afterPerspective: clock?.previousElementSibling === document.querySelector('.topbar-phase-perspective'),
        text: clock?.textContent,
        onOneLine: clockBox && box(status) && Math.abs(clockBox.top - box(status).top) < 2,
        clearOfDecisionHost: !hostBox || hostBox.right <= clockBox.left,
      };
    });
    assert.equal(placement.insideStatus, true);
    assert.equal(placement.afterPerspective, true, 'the clocks follow the "Playing as" control');
    assert.equal(placement.onOneLine, true, 'the clocks do not wrap the status row');
    assert.equal(placement.clearOfDecisionHost, true, 'the main decision, mulligan included, cannot reach them');
    assert.match(placement.text, /Alice P1 37:53/);
    assert.deepEqual(errors, []);
  } finally {await browser.close(); await vite.close();}
});
