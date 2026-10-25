import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';

test('stack particles batch GPU uploads, preserve wrapped records and retire idle bursts', async () => {
  const source = await readFile(new URL('../src/lib/motion/stack-fx-renderer.js', import.meta.url), 'utf8');
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.setContent('<canvas></canvas>');
    const result = await page.evaluate(async source => {
      let now = 0;
      performance.now = () => now;
      const url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
      const { StackFxRenderer } = await import(url);
      URL.revokeObjectURL(url);
      const renderer = new StackFxRenderer(document.querySelector('canvas'));
      renderer.resize(800, 600, 1);
      const { gl } = renderer;
      const uploads = [];
      const upload = gl.bufferSubData.bind(gl);
      gl.bufferSubData = (target, offset, data) => {
        uploads.push([offset, data.length]);
        upload(target, offset, data);
      };
      const emit = x => renderer.emit({ x, y: 100, vx: 1, vy: 0, life: 10000, size: 4, alpha: 1, rgb: '255, 100, 50' });
      const readX = () => {
        const data = new Float32Array(900 * 20);
        gl.bindBuffer(gl.ARRAY_BUFFER, renderer.particles);
        gl.getBufferSubData(gl.ARRAY_BUFFER, 0, data);
        return Array.from({ length: 900 }, (_, i) => data[i * 20]);
      };
      for (let i = 0; i < 890; i++) emit(i);
      const beforeRender = uploads.length;
      renderer.begin(); renderer.render(now);
      const first = uploads.splice(0);
      renderer.render(++now);
      const quietFrame = uploads.length;
      for (let i = 890; i < 910; i++) emit(i);
      renderer.render(now);
      const wrapped = uploads.splice(0);
      const wrappedX = readX();
      for (let i = 910; i < 2110; i++) emit(i);
      renderer.render(++now);
      const saturated = uploads.splice(0);
      const saturatedX = readX();
      now = 10002;
      emit(42); renderer.render(now);
      const idle = { count: renderer.count, uploads: uploads.splice(0), x: readX()[0] };
      const error = gl.getError();
      renderer.dispose();
      return { beforeRender, first, quietFrame, wrapped, wrappedX, saturated, saturatedX, idle, error };
    }, source);
    assert.equal(result.beforeRender, 0, 'emission performs no GPU calls');
    assert.deepEqual(result.first, [[0, 890 * 20]]);
    assert.equal(result.quietFrame, 0, 'GPU simulation needs no playback uploads');
    assert.deepEqual(result.wrapped, [[890 * 20 * 4, 10 * 20], [0, 10 * 20]]);
    assert.deepEqual(result.wrappedX, Array.from({ length: 900 }, (_, i) => i < 10 ? 900 + i : i));
    assert.deepEqual(result.saturated, [[0, 900 * 20]], 'over-capacity bursts upload only the latest ring contents');
    const expected = Array(900);
    for (let i = 1210; i < 2110; i++) expected[i % 900] = i;
    assert.deepEqual(result.saturatedX, expected);
    assert.deepEqual(result.idle, { count: 1, uploads: [[0, 20]], x: 42 });
    assert.equal(result.error, 0);
  } finally {
    await browser.close();
  }
});
