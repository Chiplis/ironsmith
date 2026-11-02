import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

for (const fallback of [false, true]) test(`all shader paths warm before effects, fallback=${fallback}`, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch({ args: ['--enable-unsafe-swiftshader'] });
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'warning' && /shader warmup failed/.test(message.text())) errors.push(message.text()); });
    await page.addInitScript(fallback => {
      localStorage.setItem('ironsmith.graphicsQuality', 'high');
      window.shaderStats = { links: 0, uniforms: 0, zoneUniforms: 0, stackUniforms: 0, polls: 0, draws: 0 };
      for (const prototype of [WebGLRenderingContext.prototype, WebGL2RenderingContext.prototype]) {
        const extension = prototype.getExtension;
        prototype.getExtension = function(name) { return fallback && name === 'KHR_parallel_shader_compile' ? null : extension.call(this, name); };
        for (const [method, stat] of [['linkProgram', 'links'], ['getUniformLocation', 'uniforms'], ['drawElements', 'draws']]) {
          const original = prototype[method];
          prototype[method] = function(...args) { window.shaderStats[stat]++;
            if (stat === 'uniforms' && this.canvas.classList.contains('zone-move-effects-canvas')) window.shaderStats.zoneUniforms++;
            if (stat === 'uniforms' && this.canvas.classList.contains('stack-fx-canvas')) window.shaderStats.stackUniforms++;
            return original.apply(this, args); };
        }
        const parameter = prototype.getProgramParameter;
        prototype.getProgramParameter = function(program, key) { if (key === 0x91b1) window.shaderStats.polls++; return parameter.call(this, program, key); };
      }
    }, fallback);
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/shader-warmup.html`);
    await page.waitForFunction(() => window.shaderStats.links >= 3 && window.shaderStats.zoneUniforms >= 14 && window.shaderStats.stackUniforms >= 3 && window.shaderStats.draws > 0);
    assert.equal(await page.locator('.zone-move-effects-layer').count(), 0, 'warm canvas does not mark an active flight');
    assert.equal(await page.locator('.zone-move-shader-layer canvas').count(), 1);
    assert.equal(await page.locator('.stack-fx-canvas').count(), 1);
    assert.equal(await page.locator('.stack-fx-canvas').evaluate(canvas => canvas.style.display), 'none');
    if (fallback) assert.equal(await page.evaluate(() => window.shaderStats.polls), 0);
    const beforeRestore = await page.evaluate(() => ({ ...window.shaderStats }));
    await page.evaluate(() => {
      window.lostShaderContexts = ['.zone-move-effects-canvas', '.stack-fx-canvas'].map((selector, index) => {
        const canvas = document.querySelector(selector);
        const gl = canvas.getContext(index ? 'webgl2' : 'webgl');
        const extension = gl.getExtension('WEBGL_lose_context');
        canvas.addEventListener('webglcontextlost', () => { canvas.dataset.lost = 'true'; }, { once: true });
        extension.loseContext();
        return extension;
      });
    });
    await page.waitForFunction(() => document.querySelectorAll('canvas[data-lost="true"]').length === 2);
    await page.evaluate(() => window.lostShaderContexts.forEach(extension => extension.restoreContext()));
    await page.waitForFunction(before => window.shaderStats.zoneUniforms > before.zoneUniforms && window.shaderStats.stackUniforms > before.stackUniforms, beforeRestore);
    await page.evaluate(() => window.effectsRoot.unmount());
    assert.equal(await page.locator('.zone-move-shader-layer canvas').count(), 0);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
