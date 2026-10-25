import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { chromium } from 'playwright';
import { build, preview } from 'vite';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';

test('material preparation runs off-thread, preserves pixels and shared sources, and falls back if workers are blocked', { timeout: 60000 }, async () => {
  const outDir = await mkdtemp(path.join(tmpdir(), 'card-glint-worker-'));
  const reference = path.resolve('node_modules/.cache/card-glint/source/.ironsmith/material-raster.ts');
  await build({ configFile: false, base: './', logLevel: 'error',
    resolve: { alias: { '@ironsmith/card-glint-material': reference } },
    build: { outDir, copyPublicDir: false, lib: { entry: { client: path.resolve('scripts/card-glint/material-worker-client.mjs'), reference }, formats: ['es'], fileName: (_, name) => `${name}.js` } } });
  await writeFile(path.join(outDir, 'index.html'), '<!doctype html><title>Material worker test</title>');
  const vite = await preview({ configFile: false, logLevel: 'error', build: { outDir }, preview: { host: '127.0.0.1', port: 0 } });
  const browser = await chromium.launch();
  try {
    for (const blocked of [false, true]) {
      const page = await browser.newPage();
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/`);
      const result = await page.evaluate(async blocked => {
        const { buildMaterialOffThread } = await import('/client.js');
        const { buildMaterial } = await import('/reference.js');
        const canvas = document.createElement('canvas'); canvas.width = 256; canvas.height = 384;
        const c = canvas.getContext('2d', { willReadFrequently: true });
        const gradient = c.createLinearGradient(0, 0, 256, 384);
        gradient.addColorStop(0, '#fff4da'); gradient.addColorStop(.5, '#427987'); gradient.addColorStop(1, '#111822');
        c.fillStyle = gradient; c.fillRect(0, 0, 256, 384);
        const source = await createImageBitmap(canvas);
        c.clearRect(0, 0, 256, 384); c.fillStyle = '#fff'; c.fillRect(40, 40, 170, 260);
        const mask = await createImageBitmap(canvas);
        const images = new Map([['mask', mask]]);
        const adjustment = { frame: { hueDegrees: 18, saturationScale: .7, brightnessScale: .9 },
          paper: { hueDegrees: -8, saturationScale: .8, brightnessScale: 1.03 },
          mask: { kind: 'rect', rect: [.1, .1, .8, .8], feather: .02 },
          blackPoint: { lift: [.01, .015, .02], transition: .3, textureMaskSource: 'mask' },
          blend: { maskSource: 'mask', outerMaskSource: 'mask', outset: .005, backdrop: [.4, .4, .4], amount: .2 } };
        const tone = { paper: [.9, .85, .8], sourceInk: [.05, .05, .05], inkTarget: [.1, .08, .06] };
        let workers = 0, mainCanvases = 0;
        const NativeWorker = window.Worker, create = document.createElement.bind(document);
        window.Worker = class extends NativeWorker {
          constructor(...args) { if (blocked) throw new Error('Workers blocked'); super(...args); workers++; }
        };
        document.createElement = (name, ...args) => { if (name === 'canvas') mainCanvases++; return create(name, ...args); };
        let actual;
        try { actual = await Promise.all([1, 2, 3].map(() => buildMaterialOffThread(source, adjustment, images, tone))); }
        finally { document.createElement = create; window.Worker = NativeWorker; }
        const expected = await buildMaterial(source, adjustment, images, tone);
        const pixels = bitmap => { c.clearRect(0, 0, 256, 384); c.drawImage(bitmap, 0, 0); return c.getImageData(0, 0, 256, 384).data; };
        const reference = pixels(expected);
        let difference = 0;
        for (const bitmap of actual) {
          const data = pixels(bitmap);
          for (let i = 0; i < data.length; i++) difference = Math.max(difference, Math.abs(data[i] - reference[i]));
          bitmap.close();
        }
        const sourceDimensions = [source.width, source.height, mask.width, mask.height];
        source.close(); mask.close(); expected.close();
        return { workers, mainCanvases, difference, sourceDimensions };
      }, blocked);
      assert.equal(result.workers, blocked ? 0 : 1);
      assert.equal(result.mainCanvases, blocked ? 3 : 0, 'worker preparation must not raster/read back on the UI thread');
      assert.ok(result.difference <= 2, `material pixels differ by ${result.difference}`);
      assert.deepEqual(result.sourceDimensions, [256, 384, 256, 384]);
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); await rm(outDir, { recursive: true, force: true }); }
});
