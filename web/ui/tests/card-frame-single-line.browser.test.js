import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('name and type stay on one line and refit around content, resizing, and fonts', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-single-line.html`);
    await page.evaluate(() => document.fonts.ready);
    const settle = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const measure = async () => {
      await settle();
      const metrics = await page.locator('.interactive-card-frame-stage').evaluate(stage => {
        const rect = selector => stage.querySelector(selector)?.getBoundingClientRect().toJSON();
        return {
          mana: rect('.interactive-card-frame__mana'), count: rect('.interactive-card-frame__count'),
          lines: ['title', 'type'].map(kind => {
            const el = stage.querySelector(`.interactive-card-frame__${kind}`);
            const range = document.createRange(); range.selectNodeContents(el);
            return { kind, text: el.textContent.trim(), size: parseFloat(getComputedStyle(el).fontSize),
              box: el.getBoundingClientRect().toJSON(), ink: range.getBoundingClientRect().toJSON(),
              lines: new Set([...range.getClientRects()].map(rect => rect.y)).size };
          }),
        };
      });
      for (const line of metrics.lines) {
        assert.equal(line.lines, 1, `${line.kind} is a single line`);
        assert.ok(line.size > 0, `${line.kind} stays visible`);
        assert.ok(line.ink.width <= line.box.width + 0.05, JSON.stringify(line));
        assert.ok(line.ink.right <= line.box.right + 0.05, `${line.kind} fits without clipping`);
      }
      assert.ok(metrics.lines[0].ink.left >= metrics.count.right + 5.9, 'count retains its space');
      if (metrics.mana) assert.ok(metrics.lines[0].ink.right <= metrics.mana.left - 7.9, 'mana retains its space');
      return metrics.lines.map(line => line.size);
    };
    const wide = await measure();
    await page.locator('#panel-host').evaluate(el => { el.style.width = '240px'; });
    const narrow = await measure();
    narrow.forEach((size, i) => assert.ok(size < wide[i], 'both lines shrink at narrow widths'));
    await page.getByRole('button', { name: 'Toggle mana' }).click();
    const noMana = await measure();
    assert.ok(noMana[0] > narrow[0], 'title grows into space released by mana');
    await page.getByRole('button', { name: 'Toggle mana' }).click();
    await page.locator('#panel-host').evaluate(el => { el.style.width = '420px'; });
    const restored = await measure();
    restored.forEach((size, i) => assert.ok(Math.abs(size - wide[i]) < 0.02, 'preferred sizing returns after resizing'));
    await page.getByRole('button', { name: 'Change text' }).click();
    assert.deepEqual(await measure(), [18, 14], 'short text uses the preferred printing size');
    await page.getByRole('button', { name: 'Change preferred size' }).click();
    assert.deepEqual(await measure(), [22, 18], 'new sampled sizes apply without a text change');
    await page.getByRole('button', { name: 'Change text' }).click();
    await page.locator('#panel-host').evaluate(el => { el.style.width = '240px'; });
    await measure();
    // Load a different face after fitting, without a React render. Its wider
    // glyphs must trigger another fit when the font becomes available.
    await page.evaluate(async () => {
      const face = new FontFace('Late Panel Font', 'url(/src/assets/fonts/GoudyMedieval.ttf)');
      document.fonts.add(face);
      const stage = document.querySelector('.interactive-card-frame-stage');
      stage.style.setProperty('--card-title-font', '"Late Panel Font"');
      stage.style.setProperty('--card-type-font', '"Late Panel Font"');
      await document.fonts.load('18px "Late Panel Font"');
      await document.fonts.ready;
    });
    await measure();
    await page.screenshot({ path: '/private/tmp/ironsmith-single-line-panels.png' });
  } finally { await browser.close(); await vite.close(); }
});
