import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('right rail shows a readable, clickable priority mode', {timeout:90000},async()=>{
 const server=await createServer({server:{host:'127.0.0.1',port:0,watch:null,hmr:false},logLevel:'silent'});await server.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?kind=priority`);
  const button=page.getByRole('button',{name:'Stack priority: Hold',exact:true});await button.waitFor();
  for(const label of ['H','C','P']) {
   await button.locator('span').evaluate((node,text)=>node.textContent=text,label);
   await button.click({trial:true});
   assert.equal(await button.evaluate(node=>node.scrollWidth<=node.clientWidth),true);
   const box=await button.boundingBox();assert.ok(box.x>1000 && box.x+box.width<=1440);
   const neighbor=await page.getByRole('button',{name:'Full control',exact:true}).boundingBox();
   assert.equal(box.width,neighbor.width);assert.equal(box.height,neighbor.height);
  }
  await button.locator('span').evaluate(node=>node.textContent='H');
  await page.screenshot({path:'/tmp/ironsmith-priority-right.png'});
 }finally{await browser.close();await server.close();}
});
