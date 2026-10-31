import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('grouped decision rows select distinct copies and retain player separation', {timeout:90000},async()=>{
 const server=await createServer({server:{host:'127.0.0.1',port:0,watch:null,hmr:false},logLevel:'silent'});await server.listen();const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:700,height:650}});const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/grouped-object-selection.html`);
  const rows=page.locator('[data-decision-option-object]');await rows.first().waitFor();assert.equal(await rows.count(),2);
  const grouped=rows.first();assert.equal(await grouped.locator('.decision-object-count').textContent(),'x3');
  assert.equal(await grouped.locator('.decision-object-count').evaluate(node=>node.nextElementSibling.classList.contains('decision-object-zone')),true);
  await grouped.click();await grouped.click();
  await page.getByRole('button',{name:'Remove one selected Forest'}).click();
  assert.match(await grouped.locator('.decision-object-count').textContent(),/1 selected/);
  await grouped.press('Enter');assert.match(await grouped.locator('.decision-object-count').textContent(),/2 selected/);
  const boxes=await rows.evaluateAll(nodes=>nodes.map(node=>{const r=node.getBoundingClientRect();return {x:r.x,y:r.y,right:r.right,bottom:r.bottom};}));
  assert.ok(boxes[1].y>=boxes[0].bottom);
  const remove=page.getByRole('button',{name:'Remove one selected Forest'});
  assert.ok((await remove.boundingBox()).x>=boxes[0].right-1);
  await page.screenshot({path:'/tmp/ironsmith-grouped-decision.png'});
  await page.getByRole('button',{name:/Submit/}).click();
  assert.deepEqual(JSON.parse(await page.locator('[data-command]').textContent()),{type:'select_objects',object_ids:[10,20]});
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await server.close();}
});
