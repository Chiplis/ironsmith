import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test("payment edits replan automatically, preserve exact selections, and gate Pay until the latest acknowledgement", async()=>{
  const vite=await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:850}}),errors=[];page.on("pageerror",e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`, {waitUntil:"domcontentloaded"});
    const commands=async()=>JSON.parse(await page.locator("[data-commands]").textContent());
    await page.getByText("Flashback · Kicker ×1",{exact:true}).waitFor();
    assert.equal(await page.getByRole("button",{name:"Change sources",exact:true}).count(),0);
    await page.getByRole("button",{name:"Hold engine",exact:true}).click();
    // A battlefield click pins the proposal; it must not activate or tap now.
    await page.locator('.battlefield-row-card[data-object-id="1"]').click();
    await page.getByRole("button",{name:"Pay",exact:true}).isDisabled().then(value=>assert.equal(value,true));
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).length===1);
    assert.equal((await commands())[0].response.action,"replan");
    await page.getByRole("button",{name:"Pip 2: 2 life · Auto",exact:true}).click();
    await page.getByRole("button",{name:"Remove Prism from payment",exact:true}).click();
    assert.equal((await commands()).length,1);
    await page.getByRole("button",{name:"Release engine",exact:true}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).length===2);
    const latest=(await commands()).at(-1).response;
    assert.deepEqual(latest.required_activations,[{source_id:"1",ability_index:0,color_restriction:null}]);
    assert.deepEqual(latest.excluded_source_ids,["2"]);
    assert.deepEqual(latest.required_life_pips,[1]);
    await page.waitForFunction(()=>!document.querySelector('.mana-plan-actions button:last-child').disabled);
    assert.equal((await commands()).length,2);
    await page.getByRole("button",{name:"Pay",exact:true}).click();
    assert.equal((await commands()).at(-1).response.action,"confirm");
    assert.equal((await commands()).at(-1).response.request_hash,`hash:${JSON.stringify(latest)}`);
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});

test("source menus select output without activation and reset clears planning constraints",async()=>{
  const vite=await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:390,height:844}}),errors=[];page.on("pageerror",e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`, {waitUntil:"domcontentloaded"});
    await page.getByRole("button",{name:"Choose payment for Prism",exact:true}).click();
    const menu=page.getByRole("dialog",{name:"Choose payment source"});
    await page.waitForTimeout(200);
    await menu.locator("[data-action-row]").nth(1).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).length===1);
    const command=JSON.parse(await page.locator("[data-commands]").textContent())[0];
    assert.equal(command.response.action,"replan");assert.deepEqual(command.response.required_activations[0].color_restriction,["blue"]);
    await page.getByRole("button",{name:"Reset",exact:true}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).length===2);
    const reset=JSON.parse(await page.locator("[data-commands]").textContent()).at(-1).response;
    assert.deepEqual(reset.required_activations,[]);assert.deepEqual(reset.excluded_source_ids,[]);assert.deepEqual(reset.required_life_pips,[]);
    await page.screenshot({path:"/tmp/payment-editor-mobile.png"});
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});


test("explicit activation adopts consumed pins and reset keeps the floating pool",async()=>{
  const vite=await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:850}}),errors=[];page.on("pageerror",e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`,{waitUntil:"domcontentloaded"});
    await page.locator('.battlefield-row-card[data-object-id="1"]').click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-payment]').textContent).required_activations?.some(value=>value.source_id==='1') && !document.querySelector('.mana-plan-actions button:last-child').disabled);
    await page.getByRole("button",{name:"Choose payment for Prism",exact:true}).click();
    let menu=page.getByRole("dialog",{name:"Choose payment source"});await page.waitForTimeout(200);await menu.locator("[data-action-row]").nth(1).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-payment]').textContent).required_activations?.some(value=>value.source_id==='2' && value.color_restriction?.includes('blue')) && !document.querySelector('.mana-plan-actions button:last-child').disabled);
    await page.getByRole("button",{name:"Choose payment for Prism",exact:true}).click();
    menu=page.getByRole("dialog",{name:"Choose payment source"});await page.waitForTimeout(200);await menu.locator("[data-action-row]").filter({hasText:"Activate now"}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.action==='activate');
    await page.waitForFunction(()=>!document.querySelector('.mana-plan-actions button:last-child').disabled);
    const payment=JSON.parse(await page.locator('[data-payment]').textContent());
    assert.deepEqual(payment.required_activations.map(value=>value.source_id),['1']);assert.deepEqual(payment.pool_before,{blue:1});
    await page.getByRole("button",{name:"Reset",exact:true}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.action==='replan' && JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.required_activations.length===0);
    assert.deepEqual(JSON.parse(await page.locator('[data-payment]').textContent()).pool_before,{blue:1});
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
