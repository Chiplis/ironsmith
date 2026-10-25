import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('card-glint decodes bounded SVG assets without a synchronous intermediate raster', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    const result = await page.evaluate(async () => {
      const { loadCardGlint } = await import('/src/lib/card-glint/session.mjs');
      const api = await loadCardGlint();
      const blob = new Blob([`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4000 3000">
        <defs><linearGradient id="g"><stop stop-color="#ddd"/><stop offset="1" stop-color="#478"/></linearGradient></defs>
        <rect x="100" y="100" width="3600" height="2600" rx="300" fill="url(#g)"/>
        <path d="M1000 900h1800v1000H1000z" stroke="#123" stroke-width="100" fill="#5a8"/>
      </svg>`], { type: 'image/svg+xml' });
      let canvases = 0;
      const create = document.createElement.bind(document);
      document.createElement = (name, ...args) => { if (name === 'canvas') canvases++; return create(name, ...args); };
      let bitmap;
      try { bitmap = await api.decodeCanvasImage(blob, new AbortController().signal); }
      finally { document.createElement = create; }
      const dimensions = [bitmap.width, bitmap.height];
      const pixels = image => {
        const canvas = create('canvas'); canvas.width = image.width; canvas.height = image.height;
        const context = canvas.getContext('2d', { willReadFrequently: true });
        context.drawImage(image, 0, 0);
        return context.getImageData(0, 0, canvas.width, canvas.height).data;
      };
      const actual = pixels(bitmap);
      // Compare to the previous drawImage -> bitmap path using the same bounded
      // SVG, so removing the raster copy cannot change the face's geometry/ink.
      const url = URL.createObjectURL(await api.sizedCanvasSvg(blob));
      const image = new Image(); image.src = url; await image.decode();
      const expected = pixels(image);
      let difference = 0, opaque = 0;
      for (let i = 0; i < actual.length; i++) difference = Math.max(difference, Math.abs(actual[i] - expected[i]));
      for (let i = 3; i < actual.length; i += 4) if (actual[i]) opaque++;
      bitmap.close(); image.src = ''; URL.revokeObjectURL(url);
      const controller = new AbortController(); controller.abort();
      let aborted = false;
      try { await api.decodeCanvasImage(blob, controller.signal); } catch (error) { aborted = error.name === 'AbortError'; }
      return { dimensions, canvases, difference, opaque, aborted };
    });
    assert.deepEqual(result.dimensions, [512, 384]);
    assert.equal(result.canvases, 0);
    assert.ok(result.difference <= 2, `SVG pixels must remain equivalent: ${result.difference}`);
    assert.ok(result.opaque > 100000);
    assert.equal(result.aborted, true);
  } finally { await browser.close(); await vite.close(); }
});
