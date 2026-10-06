import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const pixel = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGP4DwQACfsD/fteaysAAAAASUVORK5CYII=', 'base64');

test('only mana payment rolls up with a settling bounce; decisions still fold to one row', async () => {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1365, height: 850 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://**/*', route => /scryfall\.io|\.(jpg|png|webp)/.test(route.request().url())
      ? route.fulfill({ status: 200, contentType: 'image/png', body: pixel }) : route.abort());
    for (const scenario of ['priority', 'targets', 'targets&scenario=optional-target', 'select_options', 'mana_payment', 'attackers']) {
      const kind = scenario.split('&')[0];
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?kind=${scenario}`, { waitUntil: 'domcontentloaded', timeout: 60000 });
      const panel = page.locator('.battlefield-human-decision-panel');
      await panel.waitFor();
      await page.waitForTimeout(650);
      await page.getByRole('button', { name: 'Toggle decision', exact: true }).click();
      await page.waitForTimeout(100);
      const motion = await page.evaluate(async () => {
        [...document.querySelectorAll('button')].find(node => node.textContent === 'Toggle decision').click();
        for (let i = 0; i < 4; i++) await new Promise(requestAnimationFrame);
        const panel = document.querySelector('.battlefield-human-decision-panel');
        const animation = panel.getAnimations()[0];
        return animation ? { frames: animation.effect.getKeyframes(), translate: getComputedStyle(panel).translate, time: animation.currentTime } : null;
      });
      if (kind === 'mana_payment') {
        assert.ok(motion, 'mana payment starts a rollout');
        assert.ok(motion.time > 0, 'mana payment animation is playing');
        assert.notEqual(motion.translate, 'none', 'mana payment container visibly moves');
        assert.equal(motion.frames[1].translate, '0px -5px');
        assert.equal(motion.frames[2].translate, '0px 2px');
        await page.waitForTimeout(650);
      } else {
        assert.equal(motion, null, `${kind} appears without a rollout`);
      }
      await page.waitForFunction(() => getComputedStyle(document.querySelector('.battlefield-human-decision-panel')).translate === 'none');
      assert.equal(await panel.evaluate(node => getComputedStyle(node).translate), 'none', 'container settles without an inline offset');
      if (['targets', 'select_options', 'attackers'].includes(kind)) {
        await panel.locator('.battlefield-decision-disclosure-toggle, .combat-decision-minimize').click();
        const geometry = await panel.evaluate(node => {
          const heading = node.querySelector('.action-strip-decision-toolbar').getBoundingClientRect();
          const main = node.querySelector('.decision-main-button').getBoundingClientRect();
          const controls = node.querySelector('.decision-quick-controls').getBoundingClientRect();
          const bounds = node.getBoundingClientRect();
          return { height: bounds.height, headingY: heading.y + heading.height / 2, mainY: main.y + main.height / 2, right: bounds.right, mainRight: main.right, controlsLeft: controls.left, controlsRight: controls.right };
        });
        assert.ok(geometry.height <= 72, `${kind} folds to a single row: ${JSON.stringify(geometry)}`);
        assert.ok(Math.abs(geometry.headingY - geometry.mainY) < 8, `${kind} heading and action share a row: ${JSON.stringify(geometry)}`);
        assert.ok(geometry.controlsLeft >= geometry.mainRight - 1);
        assert.ok(geometry.controlsRight <= geometry.right + 1, 'controls stay inside the panel');
        if (kind !== 'attackers') {
          const headingGeometry = await panel.evaluate(node => {
            const stage = node.querySelector('.decision-stage-chip').getBoundingClientRect();
            const toggle = node.querySelector('.battlefield-decision-disclosure-toggle').getBoundingClientRect();
            return { stageWidth: stage.width, stageRight: stage.right, toggleLeft: toggle.left };
          });
          assert.ok(headingGeometry.stageWidth === 0 || headingGeometry.stageRight <= headingGeometry.toggleLeft, `the stage label does not overlap the expand arrow: ${JSON.stringify(headingGeometry)}`);
        }
        await panel.screenshot({ path: `/private/tmp/decision-minimized-${kind}.png` });
        if (kind !== 'attackers') {
          await page.setViewportSize({ width: 1024, height: 850 });
          await page.waitForTimeout(350);
          const fits = await panel.evaluate(node => {
            const bounds = node.getBoundingClientRect();
            const button = node.querySelector('.decision-main-button').getBoundingClientRect();
            const controls = node.querySelector('.decision-quick-controls').getBoundingClientRect();
            return bounds.height <= 72 && button.left >= bounds.left && controls.right <= bounds.right;
          });
          assert.ok(fits, 'the minimized action row fits the narrow dock');
          await page.setViewportSize({ width: 1365, height: 850 });
        }
      }
    }
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?kind=mana_payment`, { waitUntil: 'domcontentloaded', timeout: 60000 });
    await page.locator('.mana-payment-editor').waitFor();
    assert.equal(await page.locator('.battlefield-human-decision-panel').evaluate(node => node.getAnimations().length), 0);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await server.close(); }
});
