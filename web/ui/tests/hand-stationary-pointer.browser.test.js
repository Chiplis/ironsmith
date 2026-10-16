import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('arrow navigation ignores stationary pointer boundary events until the mouse moves', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1365,height:900}});
    await page.route('https://**/*', route=>route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`);
    const card = page.locator('.hand-card[data-object-id="4"]');
    await card.waitFor();
    const bounds = await card.boundingBox();
    const point = {x:bounds.x+20,y:Math.min(880,bounds.y+25)};
    await page.mouse.move(point.x,point.y);
    await card.focus();
    for (const key of ['ArrowRight','ArrowRight','ArrowLeft']) {
      await page.keyboard.press(key);
      const selected = await page.evaluate(()=>document.activeElement.dataset.objectId);
      await page.evaluate(point=>{
        const other = document.querySelector('.hand-card[data-object-id="2"]');
        const scope = other.closest('[data-hand-navigation-scope]');
        // Card animations can emit boundary events without mouse movement.
        for (const type of ['pointerout','pointerleave','pointerover','pointerenter']) {
          (type==='pointerleave' ? scope : other).dispatchEvent(new PointerEvent(type,{bubbles:type!=='pointerleave',pointerType:'mouse',clientX:point.x,clientY:point.y}));
        }
        other.dispatchEvent(new PointerEvent('pointermove',{bubbles:true,pointerType:'mouse',clientX:point.x,clientY:point.y,movementX:0,movementY:0}));
        other.dispatchEvent(new MouseEvent('mouseover',{bubbles:true,clientX:point.x,clientY:point.y}));
      },point);
      await page.waitForTimeout(500);
      assert.deepEqual(await page.locator('.hand-card.keyboard-selected').evaluateAll(nodes=>nodes.map(n=>n.dataset.objectId)), [selected]);
      assert.equal(await page.locator('.hand-card.hovered').count(),0);
      assert.equal(await page.locator('html').getAttribute('data-hand-pointer-suspended'),'true');
    }
    await page.mouse.move(point.x+8,point.y);
    await page.waitForTimeout(150);
    assert.equal(await page.locator('html').getAttribute('data-hand-pointer-suspended'),null);
    assert.equal(await page.locator('.hand-card.keyboard-selected').count(),0);
  } finally { await browser.close(); await vite.close(); }
});
