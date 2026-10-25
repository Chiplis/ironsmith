import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('desktop hand selection keeps transformed cards within the reserved horizontal lane',async()=>{
  const vite=await createServer({optimizeDeps:{entries:['tests/zone-piles-table.html']},server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1440,height:900}});
    await page.route('https://**/*',route=>route.fulfill({status:404,body:''}));
    for (const count of [7, 20]) {
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&objects&hand=${count}&utilities`, {waitUntil:'domcontentloaded', timeout:90000});
    const cards=page.locator('.hand-layout-item .game-card');
    await cards.first().waitFor();
    await page.waitForTimeout(700);
    const envelope=await cards.evaluateAll(nodes=>{
      const boxes=nodes.map(n=>n.getBoundingClientRect());
      return {left:Math.min(...boxes.map(r=>r.left)),right:Math.max(...boxes.map(r=>r.right))};
    });
    const dock=page.locator('[data-human-action-dock]');
    const before=await dock.boundingBox();
    for(const index of [count-1,0,Math.floor(count/2)]) {
      await dock.evaluate(node=>{
        window.dockSamples=[];
        const end=performance.now()+650;
        const sample=()=>{const r=node.getBoundingClientRect();window.dockSamples.push({x:r.x,y:r.y,width:r.width,height:r.height});if(performance.now()<end)requestAnimationFrame(sample);};
        requestAnimationFrame(sample);
      });
      await cards.nth(index).hover({force:true});
      await page.waitForTimeout(700);
      const after=await dock.boundingBox();
      assert.deepEqual(after,before,'selecting an edge card must not move or resize the dock');
      for(const sample of await page.evaluate(()=>window.dockSamples))assert.deepEqual(sample,before,'dock stays fixed throughout the animation');
      const boxes=await cards.evaluateAll(nodes=>nodes.map(n=>{const r=n.getBoundingClientRect();return {left:r.left,right:r.right};}));
      assert.ok(boxes.every(r=>r.left>=envelope.left-20 && r.right<=envelope.right+20),JSON.stringify({index,envelope,boxes}));
    }
    }
  } finally {await browser.close();await vite.close();}
});
