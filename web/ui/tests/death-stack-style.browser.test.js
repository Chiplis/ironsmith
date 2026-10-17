import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('destruction uses stack embers and a smooth fade, then cleans up', async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:700,height:500}}), errors=[];
    page.on('pageerror',e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/death-stack-style.html`);
    await page.waitForTimeout(300);
    const fire=()=>page.evaluate(()=>window.dispatchEvent(new CustomEvent('ironsmith:zone-move-effects',{detail:{effects:[{
      id:crypto.randomUUID(),kind:'death-collapse',rect:{left:260,top:120,width:160,height:220},
      sourceCloneHtml:'<div style="width:100%;height:100%;border:3px solid #b79cff;border-radius:10px;background:linear-gradient(140deg,#7054a0,#151121);color:white;padding:18px">Destroyed creature</div>',
    }]}})));
    await fire();
    await page.locator('.zone-death-card').waitFor();
    const sample=await page.locator('.zone-death-card').evaluate(n=>{
      const animation=n.getAnimations()[0];
      animation.pause(); animation.currentTime=300;
      return {opacity:Number(getComputedStyle(n).opacity),clip:getComputedStyle(n).clipPath};
    });
    assert.ok(sample.opacity>0 && sample.opacity<1);
    assert.equal(sample.clip,'none');
    assert.equal(await page.locator('.zone-death-ash,.zone-death-soul').count(),0);
    assert.ok(await page.locator('.stack-fx-canvas').evaluate(c=>c.getContext('2d').getImageData(0,0,c.width,c.height).data.some((v,i)=>i%4===3 && v>0)));
    await page.screenshot({path:'/tmp/death-stack-style.png'});
    await page.locator('.zone-death-effect').waitFor({state:'detached'});
    await page.waitForFunction(()=>getComputedStyle(document.querySelector('.stack-fx-canvas')).display==='none');
    await page.emulateMedia({reducedMotion:'reduce'});
    await fire();
    await page.waitForTimeout(100);
    assert.equal(await page.locator('.stack-fx-canvas').evaluate(n=>getComputedStyle(n).display),'none');
    assert.deepEqual(errors,[]);
  }finally{await browser.close();await vite.close();}
});
