import test from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
import {chromium} from 'playwright';
import {readFile} from 'node:fs/promises';

test('long interactive rules keep a 14px floor and scroll to the last ability',{timeout:90000},async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage({viewport:{width:800,height:800}});
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html`,{waitUntil:'domcontentloaded'});
  await page.waitForFunction(()=>window.changeGlintCard&&window.setGlintRules);
  const printing=JSON.parse(await readFile('public/cards/marvin-murderous-mimic.json','utf8')).scryfall;
  await page.evaluate(printing=>{
   const lines=['This creature has all activated abilities of creatures you control that do not have the same name as this creature.',
    '{1}{U}{B}: Ninjutsu.', '+1: You get an emblem with "Ninjas you control get +1/+1."',
    '0: Surveil 2. Then draw a card for each opponent who lost life this turn.',
    '−2: Tap target creature. Put two stun counters on it.',
    '{1}{U}, {T}: Shuffle this creature and target creature with a stun counter on it into their owners’ libraries.',
    '{T}: Add {G}.','{U}: Untap this creature.','{U}: This creature gains flying until end of turn.',
    '{U}: This creature gains shroud until end of turn.','{1}: This creature gets +1/-1 until end of turn.',
    '{1}: This creature gets -1/+1 until end of turn.'];
   window.changeGlintCard({name:'Marvin, Murderous Mimic',mana_cost:'{2}',type_line:'Legendary Artifact Creature — Toy',power:'2',toughness:'2',oracle_text:lines.join('\n')});
   window.setGlintArt(printing.image_uris.normal);
   window.marvinRules={lines,actions:new Map(lines.slice(1).map((line,index)=>[index+1,[{id:index+1,mana_payment_available:true}]]))};
   window.setGlintRules(window.marvinRules);
  },printing);
  await page.waitForFunction(()=>['unavailable','error'].includes(document.querySelector('[data-card-glint-frame]')?.getAttribute('data-card-glint-frame')));
  const rules=page.locator('.simple-card-frame__rules');
  assert.deepEqual(await rules.locator('[data-loyalty-cost]').evaluateAll(nodes=>nodes.map(node=>node.dataset.loyaltyCost)),['+1','0','−2']);
  for(const width of [366,280]){
   await page.locator('.card-frame-presentation').evaluate((element,width)=>{element.parentElement.style.width=`${width}px`;element.parentElement.style.height=`${width*680/488}px`;},width);
   await page.waitForTimeout(200);
   const metrics=await rules.evaluate(node=>({font:parseFloat(getComputedStyle(node).fontSize),height:node.clientHeight,scroll:node.scrollHeight}));
   assert.ok(metrics.font>=14,JSON.stringify(metrics));assert.ok(metrics.scroll>metrics.height,JSON.stringify(metrics));
   const last=rules.getByRole('button').last();assert.ok(await last.evaluate(node=>parseFloat(getComputedStyle(node).fontSize))>=14);await last.scrollIntoViewIfNeeded();await last.click();
   assert.equal(await page.evaluate(()=>window.glintActivated),11);
  }
  await page.locator('.card-frame-presentation').evaluate(element=>{element.parentElement.style.width='488px';element.parentElement.style.height='680px';});
  await rules.evaluate(node=>node.scrollTop=0);await page.waitForTimeout(250);
  await page.screenshot({path:'test-results/interactive-rules-readable.png'});
  await page.evaluate(()=>{
   const actions=new Map(window.marvinRules.actions);
   actions.set(4,[{id:4,mana_payment_available:false,unavailable_reason:'No legal targets.'}]);
   window.setGlintRules({...window.marvinRules,actions});
  });
  const positive=rules.getByRole('button',{name:/Marvin.*\+1: You get/});
  const zero=rules.getByRole('button',{name:/Marvin.*0: Surveil/});
  const negative=rules.getByRole('button',{name:/Marvin.*−2: Tap/});
  await zero.locator('[data-loyalty-cost]').click();
  assert.equal(await page.evaluate(()=>window.glintActivated),3);
  assert.equal(await zero.getAttribute('aria-pressed'),'true');
  await negative.evaluate(node=>node.click());
  assert.equal(await page.evaluate(()=>window.glintActivated),3);
  await rules.evaluate(node=>{const badge=node.querySelector('[data-loyalty-cost="+1"]');node.scrollTop+=badge.getBoundingClientRect().top-node.getBoundingClientRect().top-8;});
  await negative.hover();await page.keyboard.press('Tab');await positive.focus();
  assert.equal(await positive.evaluate(node=>getComputedStyle(node).color),'rgb(179, 75, 0)');
  assert.equal(await zero.evaluate(node=>getComputedStyle(node).color),'rgb(35, 106, 50)');
  assert.equal(await negative.evaluate(node=>getComputedStyle(node).color),'rgb(181, 44, 40)');
  await page.locator('.card-frame-presentation').screenshot({path:'test-results/marvin-scrollable-loyalty.png'});
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await vite.close();}
});
