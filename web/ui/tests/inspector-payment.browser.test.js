import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('card-frame affordability stays neutral while checking, blocks unpaid actions, and preserves color choices', {timeout:90000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage({viewport:{width:1000,height:850}});
  const url=`http://127.0.0.1:${vite.httpServer.address().port}/tests/inspector-payment.html`;
  await page.goto(url,{waitUntil:'domcontentloaded'});
  await page.getByRole('button',{name:'Toggle inspector'}).click();
  const ability=page.locator('button[data-available]').first();
  await ability.waitFor();
  await page.waitForFunction(()=>window.paymentRequests.length===1);
  await ability.hover();
  assert.equal(await ability.getAttribute('data-ability-state'),'pending');
  assert.equal(await ability.isEnabled(),true);
  await page.evaluate(()=>window.resolveNextPayment());
  await page.waitForFunction(()=>document.querySelector('button[data-available]')?.getAttribute('aria-disabled')==='true');
  await ability.evaluate(button=>button.click());
  assert.equal(await page.locator('output').textContent(),'0');
  await page.getByRole('button',{name:'Toggle payment'}).click();
  await page.waitForFunction(()=>window.paymentRequests.length===2);
  await page.evaluate(()=>window.resolveNextPayment());
  await page.waitForFunction(()=>document.querySelector('button[data-available]')?.getAttribute('data-available')==='true');
  await ability.click();assert.equal(await page.locator('output').textContent(),'1');
  const requests=await page.evaluate(()=>window.paymentRequests.length);
  await page.getByRole('button',{name:'Toggle inspector'}).click();
  await page.getByRole('button',{name:'Toggle inspector'}).click();
  await ability.waitFor();assert.equal(await ability.isEnabled(),true);
  assert.equal(await page.evaluate(()=>window.paymentRequests.length),requests);
  await page.goto(`${url}?dual=1`,{waitUntil:'domcontentloaded'});
  await page.getByRole('button',{name:'Toggle inspector'}).click();
  await page.waitForFunction(()=>window.paymentRequests.length===2);
  await page.evaluate(()=>{window.resolveNextPayment();window.resolveNextPayment();});
  await page.waitForFunction(()=>{const buttons=[...document.querySelectorAll('button[data-available]')];return buttons.length===2&&buttons.every(button=>button.dataset.available==='true');});
  await page.locator('button[data-available]').nth(0).click();
  await page.locator('button[data-available]').nth(1).click();
  assert.deepEqual(await page.evaluate(()=>window.activatedAbilities),[1,0]);

 }finally{await browser.close();await vite.close();}
});
