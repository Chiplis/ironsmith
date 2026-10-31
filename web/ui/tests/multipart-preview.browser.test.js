import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('new multipart spells render a preview', {timeout:60000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false,watch:null},logLevel:'silent'});
 await vite.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:1440,height:1000}});
  const errors=[];
  page.on('pageerror', error=>errors.push(error.message));
  await page.route('**/src/lib/card-glint/session.mjs*',async route=>{
   const response=await route.fetch();
   const body=(await response.text()).replace('=> queue.render(face,options);',
    '=> globalThis.failNextCardPreview ? (globalThis.failNextCardPreview=false, Promise.reject(new Error("Injected preview failure"))) : queue.render(face,options);');
   await route.fulfill({response,body});
  });
  await page.route('https://**',route=>route.abort());
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`);
  await page.getByRole('button',{name:'Build',exact:true}).click();
  const dialog=page.getByRole('dialog');
  for(const label of ['Prepared','Omen','Adventure']) {
   await dialog.getByRole('button',{name:label,exact:true}).click();
   await page.waitForTimeout(250); // Let the draft debounce select the new face.
   await page.waitForFunction(()=>{const el=document.querySelector('[data-card-glint-live]');return el?.dataset.cardGlintLive==='error'||el?.dataset.cardGlintLive==='static'&&!el?.dataset.cardGlintRendering;});
   assert.equal(await page.locator('[data-card-glint-live]').getAttribute('data-card-glint-live'),'static',label);
  }
  await page.evaluate(()=>{globalThis.failNextCardPreview=true;});
  await dialog.getByRole('textbox',{name:'Card name',exact:true}).fill('Failed preview');
  await page.waitForFunction(()=>document.querySelector('[data-card-glint-live]')?.dataset.cardGlintLive==='error');
  await dialog.getByRole('textbox',{name:'Card name',exact:true}).fill('Recovered preview');
  await page.waitForTimeout(250);
  await page.waitForFunction(()=>{const el=document.querySelector('[data-card-glint-live]');return el?.dataset.cardGlintLive==='static'&&!el?.dataset.cardGlintRendering;});
  assert.equal(await page.getByText('Card preview unavailable. You can still edit the details.',{exact:true}).count(),0);
  assert.deepEqual(errors,[]);
 }finally {await browser.close();await vite.close();}
});
