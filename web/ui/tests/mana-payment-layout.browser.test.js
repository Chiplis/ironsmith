import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('payment panel fits beside the hand and zones, below opponent cards, and scrolls long payments', async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1600,height:1000},reducedMotion:'reduce'}),errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html?kind=mana_payment&scenario=long-payment`);
    const panel=page.locator('[data-mana-payment="true"]');await panel.waitFor({state:'visible'});
    await page.waitForTimeout(600);
    for(const width of [1600,1100]) {
      await page.setViewportSize({width,height:1000});await page.waitForTimeout(300);
      const geometry=await panel.evaluate(el=>{
        const p=el.querySelector(".mana-payment-editor").getBoundingClientRect();
        const childRects=[...el.querySelectorAll(".mana-payment-editor-header,.mana-payment-editor-cost,.mana-payment-pip-row,.mana-payment-editor-footer")].map(e=>e.getBoundingClientRect());
        const zones=[...document.querySelectorAll('[data-local-zone-piles="true"] > .zone-pile-slot')].map(e=>e.getBoundingClientRect()).filter(r=>r.width>0);
        const cards=[...document.querySelectorAll('.battlefield-panel--opponents [data-zone-anchor-player]')].flatMap(zone=>[...zone.querySelectorAll('.battlefield-row[data-bf-side="top"] .battlefield-row-card')]).map(e=>e.getBoundingClientRect());
        const pay=el.querySelector('.mana-payment-pay-button'),cancel=el.querySelector('.decision-cancel-button'),payRect=pay.getBoundingClientRect(),cancelRect=cancel.getBoundingClientRect();
        const scroll=el.querySelector('.mana-payment-editor-scroll'),s=scroll.getBoundingClientRect(),footer=el.querySelector('.mana-payment-editor-footer').getBoundingClientRect();
        return {fixedHeight:Math.abs(p.height-parseFloat(getComputedStyle(el).getPropertyValue("--dock-max-height")))<1,sharedButtonStyle:pay.classList.contains('decision-main-button') && pay.classList.contains('decision-submit-button'),cancelLeft:cancelRect.right<=payRect.left && Math.abs(cancelRect.top-payRect.top)<1,buttonsFit:cancelRect.left>=p.left && payRect.right<=p.right,childrenFit:childRects.every(r=>r.left>=p.left && r.right<=p.right),left:p.left,right:p.right,top:p.top,bottom:p.bottom,zoneLeft:Math.min(...zones.map(r=>r.left)),opponentBottom:Math.max(...cards.map(r=>r.bottom)),scrolls:scroll.scrollHeight>scroll.clientHeight,footerBelow:footer.top>=s.bottom-1};
      });
      assert.equal(geometry.fixedHeight,true,JSON.stringify(geometry));assert.equal(geometry.sharedButtonStyle,true);assert.equal(geometry.cancelLeft,true,JSON.stringify(geometry));assert.equal(geometry.buttonsFit,true,JSON.stringify(geometry));
      assert.equal(geometry.childrenFit,true,JSON.stringify(geometry));
      assert.ok(geometry.right<=geometry.zoneLeft-11,JSON.stringify(geometry));
      assert.ok(geometry.top>=geometry.opponentBottom+11,JSON.stringify(geometry));
      assert.ok(geometry.bottom<=1000-13,JSON.stringify(geometry));
      assert.equal(geometry.scrolls,true);assert.equal(geometry.footerBelow,true);
    }
    await page.setViewportSize({width:1600,height:1000});await page.waitForTimeout(300);
    await page.screenshot({path:'/private/tmp/mana-payment-implemented.png'});
    const pay=await page.locator('.mana-payment-pay-button').boundingBox();
    for(const kind of ['priority','targets']) {
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html?kind=${kind}`);
      await page.locator('[data-human-action-dock]').waitFor({state:'visible'});await page.waitForTimeout(650);
      const main=await page.locator('[data-human-action-dock] .decision-main-button').boundingBox();
      assert.ok(Math.abs(main.y-pay.y)<0.1, `${kind}: ${JSON.stringify({main,pay})}`);
      assert.equal(main.height,pay.height);
    }
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
