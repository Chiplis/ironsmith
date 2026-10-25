import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { readFile } from 'node:fs/promises';

test('native motion preserves delays, final styles, callbacks and cancellation', async () => {
  const source = await readFile(new URL('../src/lib/motion/compositor.js', import.meta.url), 'utf8');
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.setContent('<!doctype html><body></body>');
    const result = await page.evaluate(async (source) => {
      const url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
      const { animateCompositor, powerEase } = await import(url);
      URL.revokeObjectURL(url);
      const node = document.createElement('div');
      document.body.append(node);
      let completed = 0;
      const motion = animateCompositor(node, [
        { transform: 'translateX(0px)', opacity: 0 },
        { transform: 'translateX(100px)', opacity: 1 },
      ], { duration: 100, delay: 100, easing: powerEase('out', 2), onComplete: () => completed++ });
      motion.pause();
      motion.currentTime = 50;
      const duringDelay = getComputedStyle(node).opacity;
      motion.currentTime = 150;
      const middle = new DOMMatrix(getComputedStyle(node).transform).m41;
      motion.finish();
      await new Promise(resolve => setTimeout(resolve, 50));
      const afterFinish = { completed, transform: node.style.transform, animations: node.getAnimations().length };
      const cancelled = animateCompositor(node, [{ opacity: 1 }, { opacity: 0 }], {
        duration: 100, onComplete: () => completed++,
      });
      cancelled.cancel();
      await new Promise(resolve => setTimeout(resolve, 150));
      return { duringDelay, middle, afterFinish, completed, animations: node.getAnimations().length };
    }, source);
    assert.equal(result.duringDelay, '0');
    assert.ok(Math.abs(result.middle - 75) < 0.1);
    assert.equal(result.afterFinish.completed, 1);
    assert.equal(result.afterFinish.animations, 0);
    assert.match(result.afterFinish.transform, /100/);
    assert.equal(result.completed, 1);
    assert.equal(result.animations, 0);
  } finally { await browser.close(); }
});
