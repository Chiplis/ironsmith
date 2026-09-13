import test from 'node:test';
import assert from 'node:assert/strict';
import process from 'node:process';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('GPU inpainting preserves unmasked pixels, matches CPU quality, and survives device loss', {timeout: 60000}, async t => {
  const vite = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'}); await vite.listen();
  const browser = await chromium.launch({args: ['--enable-unsafe-webgpu', ...(process.platform === 'darwin' ? ['--use-angle=metal'] : [])]});
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-single-line.html`);
    const hardware = await page.evaluate(async () => {
      const adapter = await navigator.gpu?.requestAdapter();
      return adapter && !adapter.info?.isFallbackAdapter;
    });
    if (!hardware) {t.skip('Hardware WebGPU adapter unavailable'); return;}
    const result = await page.evaluate(async () => {
      const requestAdapter = navigator.gpu.requestAdapter.bind(navigator.gpu);
      navigator.gpu.requestAdapter = async (...args) => {
        const adapter = await requestAdapter(...args), requestDevice = adapter.requestDevice.bind(adapter);
        adapter.requestDevice = async (...args) => (window.__frameTestDevice = await requestDevice(...args));
        return adapter;
      };
      const {inpaintGlyphMask} = await import('/src/lib/card-frame-font-mask.js');
      const {inpaintCardFrameGpu, cardFrameGpuStats} = await import('/src/lib/card-frame-gpu.js');
      const results = [];
      let last;
      for (const pattern of ['gradient', 'two-tone', 'wide-glyphs', 'isolated']) {
        const width = 256, height = 96, data = new Uint8ClampedArray(width * height * 4), mask = new Uint8Array(width * height);
        for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
          const p = y * width + x;
          const value = pattern === 'two-tone' ? (x < width / 2 ? 150 : 220) : 195 + Math.round(x / width * 35);
          data.set([value, value - 3, value - 9, 255], p * 4);
          if (pattern === 'isolated' || ((pattern === 'wide-glyphs' ? x % 48 >= 10 && x % 48 <= 30 : x % 18 >= 7 && x % 18 <= 10) && y > 20 && y < 70)) {mask[p] = 1; data.set([20, 20, 20, 255], p * 4);}
        }
        const scan = {data, width, height}, startCpu = performance.now(), cpu = inpaintGlyphMask(scan, mask), cpuMs = performance.now() - startCpu;
        const startGpu = performance.now(), gpu = await inpaintCardFrameGpu(scan, mask), gpuMs = performance.now() - startGpu;
        let untouchedChanges = 0, maxDifference = 0, totalDifference = 0, channels = 0;
        for (let p = 0; p < mask.length; p++) for (let c = 0; c < 4; c++) {
          if (!mask[p] && gpu.data[p * 4 + c] !== data[p * 4 + c]) untouchedChanges++;
          if (mask[p] && c < 3) {const delta = Math.abs(cpu.data[p * 4 + c] - gpu.data[p * 4 + c]); maxDifference = Math.max(maxDifference, delta); totalDifference += delta; channels++;}
        }
        results.push({pattern, untouchedChanges, maxDifference, meanDifference: totalDifference / channels, cpuMs, gpuMs});
        last = {scan, mask, cpu};
      }
      const stats = cardFrameGpuStats();
      window.__frameTestDevice?.destroy();
      if (window.__frameTestDevice) await window.__frameTestDevice.lost;
      const fallback = await inpaintCardFrameGpu(last.scan, last.mask);
      return {results, stats, fallbackMatches: fallback.data.every((value, index) => value === last.cpu.data[index]), afterLoss: cardFrameGpuStats()};
    });
    assert.equal(result.stats.gpuJobs, 4, JSON.stringify(result));
    for (const entry of result.results) {
      assert.equal(entry.untouchedChanges, 0, JSON.stringify(entry));
      assert.ok(entry.meanDifference <= 2 && entry.maxDifference <= 12, JSON.stringify(entry));
    }
    assert.equal(result.fallbackMatches, true);
    assert.equal(result.afterLoss.backend, 'cpu');
    t.diagnostic(JSON.stringify(result));
  } finally {await browser.close(); await vite.close();}
});
