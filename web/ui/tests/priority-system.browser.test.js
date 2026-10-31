import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdir} from 'node:fs/promises';
import {chromium} from 'playwright';
import {createServer} from 'vite';
const output='/Users/chiplis/ironsmith/reports/priority-system';
test('priority controls, keyboard, ability rules, stops and mobile layout', {timeout:90000}, async()=>{
 const server=await createServer({server:{host:'127.0.0.1',port:0,watch:null,hmr:false},logLevel:'silent'});await server.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage({viewport:{width:1280,height:1000},deviceScaleFactor:1});
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/priority-system.html`);
  await page.getByRole('button',{name:'Priority and yields',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'Priority and yields'});await dialog.waitFor();
  await mkdir(output,{recursive:true});await page.screenshot({path:`${output}/01-priority-controls.png`});
  await page.getByLabel('Hold priority after my next spell or ability').check();
  assert.equal(await page.evaluate(()=>window.__priorityFixture.status().holdNextAction),true);
  await page.keyboard.press('5');
  assert.equal(await page.evaluate(()=>window.__priorityFixture.status().holdNextAction),false);
  await page.getByRole('button',{name:'Close priority controls'}).click();
  await page.keyboard.press('6');assert.equal(await page.evaluate(()=>window.__priorityFixture.status().yield.mode),'turn');
  await page.keyboard.press('5');assert.equal(await page.evaluate(()=>window.__priorityFixture.status().yield),null);
  await page.keyboard.press('Control');assert.equal(await page.evaluate(()=>window.__held),false);
  await page.keyboard.press('1');assert.equal(await page.evaluate(()=>window.__passed),true);
  await page.getByRole('button',{name:'Ability preferences',exact:true}).click();
  await page.getByRole('dialog',{name:'Soul Warden'}).waitFor();
  await page.screenshot({path:`${output}/02-ability-preferences.png`});
  await page.getByRole('button',{name:'Always yield to this ability',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.__priorityFixture.status().rules[0].yield),true);
  await page.getByRole('button',{name:/Bob: End:/,exact:false}).click({button:'right'});
  await page.getByRole('dialog',{name:'Priority and yields'}).waitFor();
  await page.screenshot({path:`${output}/03-phase-stops.png`});
  await page.getByRole('button',{name:'Yield until End on Bob’s turn',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.__priorityFixture.status().yield.key),'phase:End');
  await page.setViewportSize({width:390,height:844});
  await page.getByRole('button',{name:'Priority and yields',exact:true}).click();
  await page.screenshot({path:`${output}/04-mobile-controls.png`});
  assert.ok(await dialog.evaluate(el=>el.getBoundingClientRect().right<=window.innerWidth));
  await page.getByRole('button',{name:'Close priority controls'}).click();
  await page.setViewportSize({width:1280,height:1000});
  // The shared modal correctly hides background controls from the accessibility tree.
  const primary=page.locator('button.decision-main-button');
  assert.equal(await primary.locator('kbd').textContent(),'1');
  await page.getByRole('button',{name:'Priority and yields',exact:true}).click();
  assert.deepEqual(await dialog.locator('kbd').allTextContents(),['2','6','8','7','5']);
  await page.getByLabel('Keyboard preset').selectOption('classic');
  assert.deepEqual(await dialog.locator('kbd').allTextContents(),['Alt+2','Alt+6','Alt+8','Alt+7','Alt+5']);
  assert.equal(await primary.locator('kbd').textContent(),'Alt+1');
  await page.getByLabel('Keyboard preset').selectOption('mtgo');
  await page.getByRole('button',{name:'Close priority controls'}).click();
  await page.evaluate(async()=>{
    window.__priorityFixture.setState({...window.__priorityFixture.state(),decision:{kind:'select_options',choice_kind:'boolean',player:0,min:1,max:1,min_choices:1,max_choices:1,options:[{index:0,description:'No'},{index:1,description:'Yes'}]}});
  });
  const yes=page.getByRole('button',{name:'Yes',exact:true});
  const no=page.getByRole('button',{name:'No',exact:true});
  await yes.waitFor();
  assert.equal(await yes.locator('kbd').textContent(),'3');
  assert.equal(await no.locator('kbd').textContent(),'4');
  await page.keyboard.press('3');
  assert.deepEqual(await page.evaluate(()=>window.__answer),{type:'select_options',option_indices:[1]});
  await page.evaluate(async()=>{const {setHandOpen}=await import('/src/lib/hand-open.js');setHandOpen(true);});
  await page.waitForFunction(()=>!document.querySelector('.priority-shortcut-hint'));
  await page.evaluate(async()=>{const {setHandOpen}=await import('/src/lib/hand-open.js');setHandOpen(false);});
  await yes.locator('kbd').waitFor();
  await page.evaluate(()=>window.__priorityFixture.setState({...window.__priorityFixture.state(), decision:{
    kind:'select_options',choice_kind:'trigger_order',player:0,min:3,max:3,min_choices:3,max_choices:3,
    description:'Order triggered abilities',options:[
      {index:0,description:'Gain life',automation_identity:'life'},
      {index:1,description:'Draw a card',automation_identity:'draw'},
      {index:2,description:'Gain life',automation_identity:'life'},
    ]}}));
  const matching=page.getByRole('button',{name:'Automatically stack matching triggers',exact:true});
  await matching.waitFor();
  assert.equal(await matching.getAttribute('aria-pressed'),'false');
  assert.equal(await matching.locator('kbd').textContent(),'7');
  await page.keyboard.press('7');
  await page.waitForFunction(()=>window.__priorityFixture.status().groupTriggers);
  assert.equal(await matching.getAttribute('aria-pressed'),'true');
  assert.match(await matching.textContent(),/On/);
  await page.keyboard.press('7');
  await page.waitForFunction(()=>!window.__priorityFixture.status().groupTriggers);
  assert.match(await matching.textContent(),/Off/);
  await matching.click();
  assert.equal(await matching.getAttribute('aria-pressed'),'true');
  await page.evaluate(()=>window.__priorityFixture.setState(window.__priorityFixture.initial));
  await matching.waitFor({state:'detached'});
  await page.keyboard.press('7');
  assert.equal(await page.evaluate(()=>window.__priorityFixture.status().groupTriggers),true);
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await server.close();}
});
