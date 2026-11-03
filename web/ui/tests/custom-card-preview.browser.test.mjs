import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('ordinary card builder previews and saves a card without a scoped-reference error',{timeout:90000},async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await vite.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage();
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`);
  await page.getByRole('button',{name:'Build',exact:true}).click();
  await page.getByRole('button',{name:'New card',exact:true}).click();
  await page.getByLabel('Card name',{exact:true}).fill('Preview Regression');
  await page.getByRole('button',{name:'Creature',exact:true}).click();
  await page.getByRole('button',{name:'Sorcery',exact:true}).click();
  await page.getByLabel('Rules text',{exact:true}).fill('Destroy target creature.');
  await page.getByRole('button',{name:'Save',exact:true}).click();
  await page.waitForFunction(()=>document.body.textContent.includes('Saved Preview Regression'));
  assert.ok(!(await page.locator('body').innerText()).includes('scoped is not defined'));
 } finally {await browser.close();await vite.close();}
});
