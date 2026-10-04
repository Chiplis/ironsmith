import test from "node:test";
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { createServer } from "vite";

test('pip menus honor once-per-turn and finite-use capacities, and long plans keep Pay outside the scroll area', async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:850}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`);
    await page.getByRole('button',{name:'Limited source plan',exact:true}).click();
    await page.getByRole('button',{name:'Choose payment for Mountain',exact:true}).click();
    let menu=page.getByRole('dialog',{name:'Choose payment source'});
    assert.equal(await menu.getByText(/Wall of Roots/).count(),0);
    assert.equal(await menu.getByText(/Counter source/).count(),0);
    await page.keyboard.press('Escape');await menu.waitFor({state:'detached'});
    await page.getByRole('button',{name:'Choose payment for Counter source',exact:true}).first().click();
    menu=page.getByRole('dialog',{name:'Choose payment source'});
    assert.equal(await menu.getByText(/Counter source/).count(),1);
    assert.equal(await menu.getByText(/Wall of Roots/).count(),0);
    await page.keyboard.press('Escape');await menu.waitFor({state:'detached'});
    assert.equal(await page.locator('[data-payment-pip-id="0"] img[src$="/1.svg"]').count(),1,'row shows generic cost, not green output');
    assert.equal(await page.getByRole('button',{name:'Add payment source',exact:true}).count(),0);
    const initialHeight=await page.locator('.mana-payment-editor').evaluate(el=>el.getBoundingClientRect().height);
    await page.getByRole('button',{name:'Long payment plan',exact:true}).click();
    assert.equal(await page.locator('.mana-payment-editor').evaluate(el=>el.getBoundingClientRect().height),initialHeight);
    assert.equal(await page.locator('.mana-payment-editor-scroll').evaluate(el=>el.scrollHeight>el.clientHeight),true);
    const scroll=await page.locator('.mana-payment-editor-scroll').boundingBox(),footer=await page.locator('.mana-payment-editor-footer').boundingBox();
    assert.ok(footer.y>=scroll.y+scroll.height-1);
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});

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
    await page.getByRole("button",{name:"Choose payment for Prism",exact:true}).click();
    await page.getByRole('dialog',{name:'Choose payment source'}).getByText('Pay 2 life',{exact:true}).click();
    assert.equal((await commands()).length,1);
    await page.getByRole("button",{name:"Release engine",exact:true}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).length===2);
    const latest=(await commands()).at(-1).response;
    assert.deepEqual(latest.required_activations,[{source_id:"1",ability_index:0,color_restriction:null}]);
    assert.deepEqual(latest.excluded_source_ids,["2"]);
    assert.deepEqual(latest.required_life_pips,[1]);
    await page.waitForFunction(()=>!document.querySelector('.mana-payment-editor-footer .mana-payment-pay-button').disabled);
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


test("interactive activation adopts consumed pins and reset keeps the floating pool",async()=>{
  const vite=await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:850}}),errors=[];page.on("pageerror",e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`,{waitUntil:"domcontentloaded"});
    await page.locator('.battlefield-row-card[data-object-id="1"]').click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-payment]').textContent).required_activations?.some(value=>value.source_id==='1') && !document.querySelector('.mana-payment-editor-footer .mana-payment-pay-button').disabled);
    await page.getByRole("button",{name:"Choose payment for Prism",exact:true}).click();
    let menu=page.getByRole("dialog",{name:"Choose payment source"});await page.waitForTimeout(200);await menu.locator("[data-action-row]").nth(1).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-payment]').textContent).required_activations?.some(value=>value.source_id==='2' && value.color_restriction?.includes('blue')) && !document.querySelector('.mana-payment-editor-footer .mana-payment-pay-button').disabled);
    await page.getByRole("button",{name:"Choose payment for Prism",exact:true}).click();
    menu=page.getByRole("dialog",{name:"Choose payment source"});await page.waitForTimeout(200);await menu.locator("[data-action-row]").filter({hasText:"Activate now"}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.action==='activate');
    await page.waitForFunction(()=>!document.querySelector('.mana-payment-editor-footer .mana-payment-pay-button').disabled);
    const payment=JSON.parse(await page.locator('[data-payment]').textContent());
    assert.deepEqual(payment.required_activations.map(value=>value.source_id),['1']);assert.deepEqual(payment.pool_before,{blue:1});
    await page.getByRole("button",{name:"Reset",exact:true}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.action==='replan' && JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.required_activations.length===0);
    assert.deepEqual(JSON.parse(await page.locator('[data-payment]').textContent()).pool_before,{blue:1});
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});


test('warning icon explains payment consequences without confirming, and undo safety alone shows no warning', async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:850}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`);
    await page.getByRole('button',{name:'Undo-only plan',exact:true}).click();
    assert.equal(await page.getByRole('button',{name:'Payment warnings',exact:true}).count(),0);
    await page.getByRole('button',{name:'Warning plan',exact:true}).click();
    assert.equal(await page.getByText('Pay 2 life.',{exact:true}).count(),0);
    await page.getByRole('button',{name:'Payment warnings',exact:true}).click();
    await page.getByText('Pay 2 life.',{exact:true}).waitFor();
    await page.getByText('This plan leaves mana floating after payment.',{exact:true}).waitFor();
    assert.equal(await page.getByText(/cannot.*safely.*undo/i).count(),0);
    assert.deepEqual(JSON.parse(await page.locator('[data-commands]').textContent()),[]);
    await page.keyboard.press('Escape');
    await page.getByText('Pay 2 life.',{exact:true}).waitFor({state:'detached'});
    await page.getByRole('button',{name:'Pay',exact:true}).click();
    assert.equal(JSON.parse(await page.locator('[data-commands]').textContent()).at(-1).response.action,'confirm');
  } finally {await browser.close();await vite.close();}
});


test('Cancel uses the action rollback path, including with an outstanding replan', async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:850}});
    for(const pending of [false,true]) {
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`);
      if(pending) {
        await page.getByRole('button',{name:'Hold engine',exact:true}).click();
        await page.locator('.battlefield-row-card[data-object-id="1"]').click();
        await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.action==='replan');
      }
      await page.getByRole('button',{name:'Cancel',exact:true}).click();
      await page.locator('.mana-payment-editor').waitFor({state:'detached'});
      const commands=JSON.parse(await page.locator('[data-commands]').textContent());
      assert.deepEqual(commands.at(-1),{type:'cancel_decision',options:{waitForPaymentReady:true}});
      assert.equal(commands.some(command=>command.response?.action==='cancel'),false);
      await page.waitForTimeout(150);
      assert.equal(await page.locator('.mana-payment-editor').count(),0,'a late replan cannot reopen the cancelled payment');
    }
  } finally {await browser.close();await vite.close();}
});


test('pip menus mark the current choice and leave source preferences in advanced controls', async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:850}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`);
    await page.getByRole('button',{name:'Choose payment for Prism',exact:true}).click();
    let menu=page.getByRole('dialog',{name:'Choose payment source'});
    const current=menu.getByRole('button',{pressed:true});
    assert.equal(await current.count(),1);
    assert.match(await current.textContent(),/Prism/);
    assert.equal(await current.locator('svg').count(),1);
    assert.equal(await menu.getByText(/Keep this source|Remove this source|Prefer to save|Let the planner/).count(),0);
    assert.equal(await menu.locator('[data-action-row]').filter({hasText:'Activate now'}).count(),1);
    assert.equal(await menu.getByText(/Activate now.*Add any color/).count(),0);
    await page.waitForTimeout(200);
    await current.click();
    await menu.waitFor({state:'detached'});
    assert.deepEqual(JSON.parse(await page.locator('[data-commands]').textContent()),[],'clicking the current choice is a no-op');
    await page.getByRole('button',{name:'Advanced payment controls',exact:true}).click();
    menu=page.getByRole('dialog',{name:'Advanced payment controls'});
    await page.waitForTimeout(200);
    await menu.getByText('Prefer to save Prism',{exact:true}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.preserved_source_ids.includes('2'));
    await page.waitForFunction(()=>!document.querySelector('.mana-payment-pay-button').disabled);
    await page.getByRole('button',{name:'Advanced payment controls',exact:true}).click();
    menu=page.getByRole('dialog',{name:'Advanced payment controls'});
    await page.waitForTimeout(200);
    assert.match(await menu.getByRole('button',{pressed:true}).textContent(),/Prism/);
    await menu.getByText('Prefer to save Prism',{exact:true}).click();
    await page.waitForFunction(()=>JSON.parse(document.querySelector('[data-commands]').textContent).at(-1)?.response.preserved_source_ids.length===0);
  } finally {await browser.close();await vite.close();}
});


test("mana source options enlarge their battlefield or hand card and clear on dismissal", async()=>{
  const vite=await createServer({server:{host:"127.0.0.1",port:0},logLevel:"silent"});await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:1100}}),errors=[];page.on("pageerror",e=>errors.push(e.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`, {waitUntil:"domcontentloaded"});
    await page.getByRole("button",{name:"Choose payment for Mountain",exact:true}).click();
    const menu=page.getByRole("dialog",{name:"Choose payment source"});
    await menu.locator("[data-action-row]").filter({hasText:"Prism"}).first().hover();
    const prism=page.locator('.battlefield-row-card[data-object-id="2"].payment-source-option-hover');
    await prism.waitFor();
    assert.equal(await prism.evaluate(el=>getComputedStyle(el).scale),"1.2");
    assert.equal(await page.locator('.battlefield-row-card[data-object-id="1"].payment-source-option-hover').count(),0);
    await page.keyboard.press("Escape");await menu.waitFor({state:"detached"});
    assert.equal(await page.locator('.payment-source-option-hover').count(),0);
    await page.getByRole("button",{name:"Hand source plan",exact:true}).click();
    await page.locator('[data-hand-object-id="5"]').waitFor();
    await page.getByRole("button",{name:"Choose payment for Mountain",exact:true}).click();
    await menu.locator("[data-action-row]").filter({hasText:"Hand mana source"}).hover();
    await page.locator('.hand-card.inspected').waitFor();
    await page.keyboard.press("Escape");await menu.waitFor({state:"detached"});
    await page.waitForFunction(()=>!document.querySelector('.hand-card.inspected'));
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
