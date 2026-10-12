import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

// Control the expensive pixel sampler, retaining the real preparation cache,
// image decoding, React hook, frame selection, and stage lifecycle.
const sampler = `
import { defaultCardFrame } from '/src/lib/default-card-frames.js';
const gate = new Promise(resolve => { window.releaseFrameSampler = resolve; });
export async function sampleCardFrameColors() {
  window.frameSamplerStarted = true;
  await gate;
  if (window.frameSamplerResult === 'reject') throw new Error('Mask worker failed');
  if (window.frameSamplerResult === 'unusable') return null;
  const frame = defaultCardFrame({ typeLine: 'Creature', manaCost: '{R}' });
  const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="488" height="680"><rect width="488" height="680" fill="#dfd4bb"/></svg>';
  return { ...frame.style, '--source-frame-image': 'url("data:image/svg+xml,' + encodeURIComponent(svg) + '")' };
}
`;

test('stock frame upgrades after preparation and remains visible on slow or failed preparation', { timeout: 60000 }, async () => {
  const vite = await createServer({
    server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent',
    plugins: [{ name: 'controlled-frame-sampler', enforce: 'pre', transform(code, id) {
      if (id.endsWith('/src/lib/card-frame-processing.js')) return sampler;
    } }],
  });
  await vite.listen();
  let browser;
  try {
    browser = await chromium.launch();
    for (const result of ['success', 'unusable', 'reject']) {
      const page = await browser.newPage();
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.addInitScript(result => { window.frameSamplerResult = result; }, result);
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/default-card-frames.html?preview`);
      await page.addScriptTag({ type: 'module', url: '/tests/default-card-frames.jsx' });
      const stage = page.locator('[data-template="R"] .interactive-card-frame-stage');
      await page.waitForFunction(() => window.frameSamplerStarted);
      assert.equal(await stage.getAttribute('data-default-frame'), 'R');
      assert.equal(await stage.getAttribute('aria-hidden'), 'false');
      assert.match(await stage.innerText(), /Vigilance/);
      await stage.evaluate(node => {
        window.frameHiddenDuringUpgrade = false;
        new MutationObserver(records => {
          if (records.some(record => record.oldValue === 'true') || node.getAttribute('aria-hidden') === 'true') {
            window.frameHiddenDuringUpgrade = true;
          }
        }).observe(node, { attributes: true, attributeFilter: ['aria-hidden'], attributeOldValue: true });
      });
      await page.evaluate(() => window.releaseFrameSampler());
      if (result === 'success') {
        await page.waitForFunction(() => {
          const node = document.querySelector('[data-template="R"] .interactive-card-frame-stage');
          return !node.hasAttribute('data-default-frame') && node.dataset.renderReady === 'true';
        });
        assert.equal(await stage.getAttribute('data-frame-mode'), 'masked');
        assert.match(await stage.evaluate(node => node.style.getPropertyValue('--source-frame-image')), /data:image\/svg\+xml/);
      } else {
        // Wait for the actual preparation promise, then let React publish it.
        await page.evaluate(async () => {
          const { prepareCardFrame } = await import('/src/lib/card-frame-preparation.js');
          const image = document.querySelector('[data-template="R"] .interactive-card-frame__art img');
          await prepareCardFrame(image.src, 'Creature — Test').catch(() => {});
          await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        });
        assert.equal(await stage.getAttribute('data-default-frame'), 'R');
      }
      assert.equal(await stage.getAttribute('aria-hidden'), 'false');
      assert.equal(await page.evaluate(() => window.frameHiddenDuringUpgrade), false);
      assert.match(await stage.innerText(), /Vigilance/);
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally { await browser?.close(); await vite.close(); }
});
