import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('opening hand shortcut and Exalted inspector preferences', {timeout:120000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});await vite.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage({viewport:{width:1200,height:1000}});const errors=[];
  page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(()=>localStorage.setItem('ironsmith.priorityShortcutPreset','mtgo'));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/oct31-bugs.html`);
  const keep=page.getByRole('button',{name:'Keep hand',exact:true});await keep.waitFor();
  await page.keyboard.press('1');
  assert.equal(await page.evaluate(()=>window.calls[0]?.action_ref?.kind),'keep_opening_hand');
  const mulligan=page.getByRole('button',{name:'Mulligan',exact:true});
  await mulligan.click();assert.equal(await page.evaluate(()=>window.calls[1]?.action_ref?.kind),'take_mulligan');
  await page.locator('[data-ability-text="Exalted"]:visible').first().click({button:'right'});
  await page.getByRole('dialog',{name:'Ignoble Hierarch'}).waitFor();
  await page.getByRole('button',{name:'Always wait for this ability',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.policy.status(window.fixtureState).rules[0].yield),false);
  await page.locator('[data-ability-text="Exalted"]:visible').first().click({button:'right'});
  await page.getByRole('button',{name:'Always yield to this ability',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.policy.status(window.fixtureState).rules[0].yield),true);
  await page.getByRole('button',{name:'Ability priority preferences',exact:true}).click();
  await page.getByRole('button',{name:'Exalted',exact:true}).click();
  await page.getByRole('button',{name:'Countdown before passing this ability',exact:true}).click();
  assert.equal(await page.evaluate(()=>window.policy.status(window.fixtureState).rules[0].yield),'countdown');
  await page.getByRole('button',{name:'Diagnostics',exact:true}).click();
  await page.getByRole('button',{name:'Load diagnostic',exact:true}).click({trial:true});
  const input=page.getByLabel('Diagnostic report file');
  await input.setInputFiles({name:'invalid.json',mimeType:'application/json',buffer:Buffer.from('{}')});
  await page.getByRole('alert').filter({hasText:'not an Ironsmith'}).waitFor();
  await input.setInputFiles({name:'christian.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({
    exportedAt:'2026-10-31',mainThread:{lagMs:0},events:[],traces:[],gameState:{turn_number:7,decision:{kind:'priority'}},journal:{entries:[]}
  }))});
  await page.getByRole('heading',{name:'christian.json'}).waitFor();
  await page.getByText('gameState',{exact:true}).click();
  assert.match(await page.locator('details[open] pre').textContent(),/turn_number.*7/s);
  await page.getByRole('button',{name:'Back to live',exact:true}).click();
  await page.getByRole('heading',{name:'This browser',exact:true}).waitFor();
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await vite.close();}
});

test('built renderer retains land stock and two-color pinlines', {timeout:90000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});await vite.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage({viewport:{width:440,height:570}});
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html`);
  await page.waitForFunction(()=>window.changeGlintCard);
  const profiles=await page.evaluate(async()=>{
   const {loadCardGlint}=await import('/src/lib/card-glint/session.mjs');
   const api=await loadCardGlint();
   return [['U'],['U','R'],['W','U','B']].map(frameColors=>api.resolveCardProfile({typeLine:'Land',land:true,frameColors}));
  });
  assert.deepEqual(profiles.map(profile=>profile.boxes),['Land','Land','Land']);
  assert.deepEqual(profiles.map(profile=>profile.frame),['U','UR','Gold']);
  await page.evaluate(()=>{
   window.setGlintRules({lines:['{T}: Add {U} or {R}.'],actions:new Map()});
   window.changeGlintCard({name:'Dual Land Fixture',type_line:'Land',colors:[],color_identity:['U','R'],oracle_text:'{T}: Add {U} or {R}.',set:'m21'});
  });
  await page.getByRole('img',{name:/Dual Land Fixture/}).waitFor();
  await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:60000});
  await page.screenshot({path:'/tmp/ironsmith-oct31-dual-land.png'});
 }finally{await browser.close();await vite.close();}
});
