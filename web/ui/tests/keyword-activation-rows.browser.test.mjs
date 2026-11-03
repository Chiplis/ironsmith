import test from 'node:test';import assert from 'node:assert/strict';
import {createServer} from 'vite';import {chromium} from 'playwright';
test('Improve, Advance and Equip X rows activate while keyword and mana clicks remain helpers',{timeout:90000},async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await vite.listen();const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:1200,height:900}});
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/keyword-activation-rows.html`);
  await page.waitForFunction(()=>document.querySelectorAll('[data-card-glint-frame="ready"]').length===4);
  for(const width of [488,260]) {
   await page.locator('[data-example]').evaluateAll((nodes,width)=>nodes.forEach(node=>{node.style.width=`${width}px`;node.style.height=`${width*680/488}px`;}),width);
  for(const [id,symbol] of [['improve','B'],['advance','B'],['compiled','B'],['equip','X']]) {
   const section=page.locator(`[data-example=${id}]`);
   const row=section.locator('button.mse-card-hit-target, button.inspector-oracle-line-action');
   assert.equal(await row.count(),1);
   const before=await page.evaluate(()=>window.rowActivations.length);
   await section.getByRole('button',{name:new RegExp(`^${id==='compiled'?'improve':id} rules helper$`,'i')}).click();
   await page.locator('[data-ui-layer=tooltip][data-state=open]').waitFor();
   assert.equal(await page.evaluate(()=>window.rowActivations.length),before);
   await page.keyboard.press('Escape');
   await section.locator(`[aria-label="{${symbol}}"], [aria-label="${symbol==='B'?'Black Mana':'Variable Mana'}"]`).first().click();
   await page.locator('[data-ui-layer=tooltip][data-state=open]').waitFor();
   assert.equal(await page.evaluate(()=>window.rowActivations.length),before);
   await page.keyboard.press('Escape');
   await row.scrollIntoViewIfNeeded();
   const position=await row.evaluate(node=>{
    const box=node.getBoundingClientRect();const helpers=[...node.closest('[data-example]').querySelectorAll('[aria-haspopup="dialog"]')].map(el=>el.getBoundingClientRect());
    for(const y of [0.5,0.8,0.2])for(const x of [0.95,0.75,0.5,0.25]){
     const px=box.x+box.width*x,py=box.y+box.height*y;
     if(document.elementFromPoint(px,py)===node && !helpers.some(r=>px>=r.left&&px<=r.right&&py>=r.top&&py<=r.bottom))return {x:px,y:py};
    }
   });assert.ok(position,'The action row must have a clickable area outside its helpers');await page.mouse.click(position.x,position.y);
   assert.equal(await page.evaluate(()=>window.rowActivations.length),before+1);
   assert.equal(await page.evaluate(()=>window.rowActivations.at(-1)),id);
  }
  }
 }finally{await browser.close();await vite.close();}
});
