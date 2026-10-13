import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('all standard color sets render local frames with live art and text, including complete Saga artwork', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  let browser;
  try {
    browser = await chromium.launch();
    const page = await browser.newPage({ viewport: { width: 1080, height: 900 } });
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    const network = [];
    await page.route(/https:\/\/[^/]*scryfall[^/]*\//, route => { network.push(route.request().url()); return route.abort(); });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/default-card-frames.html?preview`);
    await page.addScriptTag({ type: 'module', url: '/tests/default-card-frames.jsx' });
    await page.waitForFunction(() => document.querySelectorAll('[data-default-frame][data-render-ready="true"]').length === 36, null, { timeout: 30000 });
    assert.deepEqual(errors, []);
    for (const [key, color] of [['saga', 'G'], ['urzas-saga', 'land']]) {
      const frame = page.locator(`[data-template="${key}"] [data-default-frame]`);
      assert.equal(await frame.getAttribute('data-default-frame'), color);
      const geometry = await frame.evaluate(async node => {
        const img = node.querySelector('.interactive-card-frame__art img');
        await img.decode();
        const art = img.getBoundingClientRect();
        const title = node.querySelector('.interactive-card-frame__title-row').getBoundingClientRect();
        const style = getComputedStyle(img);
        return { fit: style.objectFit, top: art.top, titleBottom: title.bottom,
          width: art.width, height: art.height, ratio: img.naturalWidth / img.naturalHeight };
      });
      assert.equal(geometry.fit, 'contain');
      assert.ok(geometry.top >= geometry.titleBottom - 1, JSON.stringify(geometry));
      assert.ok(geometry.ratio < geometry.width / geometry.height, 'fixture must exercise a tall art crop');
    }
    assert.equal(await page.locator('[data-default-frame] .interactive-card-frame__art img').count(), 36);
    assert.equal(await page.locator('[data-default-frame]').filter({ hasText: 'Vigilance' }).count(), 34);
    assert.deepEqual(network, []);
    await mkdir('test-results/default-card-frames', { recursive: true });
    for (let index = 0; index < 5; index++) {
      await page.evaluate(index => {
        [...document.querySelectorAll('[data-template]')].forEach((node, i) => { node.style.display = Math.floor(i / 8) === index ? '' : 'none'; });
      }, index);
      await page.screenshot({ path: `test-results/default-card-frames/frames-${index}.png`, fullPage: true });
    }
  } finally { await browser?.close(); await vite.close(); }
});

test('relative production bases load frames beside the app and retain readable paper on delay or failure', { timeout: 60000 }, async () => {
  const css = await readFile(new URL('../src/styles/card-frame-colors.css', import.meta.url), 'utf8');
  const vite = await createServer({
    server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent',
    plugins: [{
      name: 'relative-production-frame-base', enforce: 'pre',
      transform(code, id) {
        if (id.endsWith('/src/lib/default-card-frames.js')) {
          return code.replace('import.meta.env?.BASE_URL', JSON.stringify('./'));
        }
      },
    }],
  });
  await vite.listen();
  let browser;
  try {
    browser = await chromium.launch();
    for (const fail of [false, true]) {
      const page = await browser.newPage();
      const requests = [];
      let release;
      const pending = new Promise(resolve => { release = resolve; });
      await page.route('**/mounted/index.html', async route => route.fulfill({
        contentType: 'text/html', body: await vite.transformIndexHtml('/mounted/index.html', '<!doctype html><html><head></head><body><div id="frames"></div></body></html>'),
      }));
      // Match production: the consuming stylesheet lives one directory below
      // the document, and Vite supplies a relative BASE_URL.
      await page.route('**/mounted/assets/frame.css', route => route.fulfill({ contentType: 'text/css', body: css }));
      await page.route('**/card-frames/default/*.webp', async route => {
        requests.push(new URL(route.request().url()).pathname);
        await pending;
        if (fail) return route.abort();
        const name = new URL(route.request().url()).pathname.split('/').pop();
        return route.fulfill({ contentType: 'image/webp', body: await readFile(new URL(`../public/card-frames/default/${name}`, import.meta.url)) });
      });
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/mounted/index.html`);
      await page.addScriptTag({ type: 'module', url: '/tests/default-card-frames.jsx' });
      await page.waitForSelector('[data-default-frame]');
      await page.addStyleTag({ url: '/mounted/assets/frame.css' });
      const frame = page.locator('[data-template="R"] .interactive-card-frame');
      const checkPaper = async () => {
        const background = await frame.evaluate(node => getComputedStyle(node).backgroundColor);
        assert.match(background, /^rgb\(/, `expected opaque fallback, got ${background}`);
        const channels = background.match(/[\d.]+/g).map(Number);
        assert.ok(Math.min(...channels) > 150, `fallback must contrast with dark ink: ${background}`);
        assert.equal(await page.locator('[data-template="R"] .interactive-card-frame-stage').getAttribute('aria-hidden'), 'false');
      };
      await checkPaper();
      release();
      assert.ok(requests.length > 0);
      assert.ok(requests.every(path => path.startsWith('/mounted/card-frames/default/')), requests.join('\n'));
      const decoded = await frame.evaluate(async node => {
        const url = getComputedStyle(node).backgroundImage.match(/url\("([^"]+)"\)/)[1];
        const image = new Image(); image.src = url;
        try { await image.decode(); return true; } catch { return false; }
      });
      assert.equal(decoded, !fail);
      await checkPaper();
      await page.close();
    }
  } finally { await browser?.close(); await vite.close(); }
});
