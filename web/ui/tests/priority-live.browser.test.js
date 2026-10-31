import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdir} from 'node:fs/promises';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('live table exposes priority controls and arms hold-next-action',{timeout:120000},async()=>{
 const server=await createServer({define:{'import.meta.env.VITE_E2E_TEST':JSON.stringify('true')},server:{host:'127.0.0.1',port:0,watch:null,hmr:false},logLevel:'silent'});await server.listen();
 const browser=await chromium.launch({args:['--enable-unsafe-swiftshader']});
 try{
  const page=await browser.newPage({viewport:{width:1500,height:1000},deviceScaleFactor:1});
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(()=>localStorage.setItem('ironsmith.showShortcutsOnStart','false'));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/`);
  await page.waitForSelector('[data-workspace-shell]',{timeout:90000});
  await page.waitForFunction(()=>window.__ironsmithE2E?.snapshot()?.state?.decision, null, {timeout:90000});
  await page.screenshot({path:'../../reports/priority-system/00-live-before-controls.png'});
  await page.getByRole('button',{name:'Priority and yields',exact:true}).first().click({timeout:60000});
  await page.getByRole('dialog',{name:'Priority and yields'}).waitFor();
  await mkdir('../../reports/priority-system',{recursive:true});
  await page.screenshot({path:'../../reports/priority-system/05-live-table-controls.png'});
  await page.getByLabel('Hold priority after my next spell or ability').check();
  await page.getByRole('button',{name:'Close priority controls'}).click();
  await page.getByRole('button',{name:'Hold next priority ×',exact:true}).first().waitFor();
  await page.screenshot({path:'../../reports/priority-system/06-live-table.png'});
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await server.close();}
});
