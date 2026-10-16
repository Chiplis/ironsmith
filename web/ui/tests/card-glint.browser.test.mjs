import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('card-glint paints a real Magic face and keeps live ability actions', {timeout:180000}, async () => {
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:440,height:570}}), errors=[];
    page.on('pageerror',error=>errors.push(error.message));
    page.on('console',message=>{if(message.type()==='error') console.error(message.text());});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.waitForFunction(()=>window.warmGlintFrame);
    assert.equal(await page.locator('[data-card-glint-frame]').count(),0);
    await page.evaluate(()=>window.warmGlintFrame());
    await page.evaluate(()=>window.showGlintFrame());
    await page.waitForFunction(() => ['ready','error','unavailable'].includes(document.querySelector('[data-card-glint-frame]')?.dataset.cardGlintFrame), undefined, {timeout:150000});
    assert.equal(await page.locator('[data-card-glint-frame]').getAttribute('data-card-glint-frame'),'ready');
    assert.ok(await page.locator('[data-card-glint-frame]').evaluate(canvas=>canvas.toDataURL()===window.warmedGlintResult.canvas.toDataURL()),'mounted frame uses the prepared pixels');
    const button=page.getByRole('button',{name:'Glint Test Adept: {T}: Draw a card.'});
    await button.first().click();
    assert.equal(await page.evaluate(()=>window.glintActivated),'draw');
    const pixels=await page.locator('[data-card-glint-frame]').evaluate(canvas=>{
      const data=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
      let opaque=0; for(let i=3;i<data.length;i+=4) if(data[i]>0) opaque++;
      return opaque/(canvas.width*canvas.height);
    });
    assert.ok(pixels>.8,'actual frame pixels are painted');
    await page.screenshot({path:'test-results/card-glint.png'});
    await page.evaluate(()=>window.changeGlintCard({name:'Changed Adept',type_line:'Creature — Wizard',colors:['R'],
      oracle_text:'{T}: Add {R}.',power:'5',toughness:'5'}));
    await page.getByRole('img',{name:/Changed Adept/}).waitFor();
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:30000});
    assert.equal(await page.getByRole('button',{name:/Glint Test Adept/}).count(),0);
    await page.getByRole('button',{name:'Changed Adept: {T}: Add {R}.'}).first().click();
    await page.evaluate(() => {
      const art=document.createElement('canvas'); art.width=100; art.height=100;
      const context=art.getContext('2d'); context.fillStyle='#e02020'; context.fillRect(0,0,100,100);
      window.setGlintArt(art.toDataURL());
    });
    await page.waitForFunction(() => {
      const canvas=document.querySelector('[data-card-glint-frame="ready"]');
      if(!canvas) return false;
      const [r,g,b]=canvas.getContext('2d').getImageData(canvas.width/2,canvas.height/3,1,1).data;
      return r>180 && g<70 && b<70;
    });
    assert.deepEqual(errors,[]);
    // A failed renderer module keeps text and ability buttons usable.
    await page.route('**/card-glint/**/magic-presentation.js',route=>route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html`);
    await page.locator('[data-card-glint-frame="error"]').waitFor({state:'attached'});
    await page.locator('.simple-card-frame').getByRole('button',{name:'Glint Test Adept: {T}: Draw a card.'}).click();
    assert.equal(await page.evaluate(()=>window.glintActivated),'draw');
  } finally {await browser.close();await vite.close();}
});

test('initial hand and battlefield warm without mounted frames and follow live state changes', {timeout:180000},async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});await vite.listen();
  const browser=await chromium.launch();
  try{
    const page=await browser.newPage(),errors=[];
    page.on('pageerror',error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint-background.html`);
    await page.waitForFunction(()=>window.glintBackgroundPending?.(),undefined,{timeout:60000});
    assert.equal(await page.locator('[data-card-glint-frame]').count(),0);
    await page.evaluate(()=>window.finishGlintBackground());
    await page.evaluate(()=>window.changeGlintBackground());
    await page.waitForFunction(()=>window.glintBackgroundPending());
    await page.evaluate(()=>window.finishGlintBackground());
    await page.evaluate(()=>window.showGlintBackground());
    await page.locator('[data-card-glint-frame="ready"]').waitFor();
    assert.deepEqual(errors,[]);
  }finally{await browser.close();await vite.close();}
});

test('default frames render custom colors and special card types without a printing', {timeout:120000}, async () => {
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1260,height:900}});
    await page.route('https://api.scryfall.com/**',route=>route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    const rendered=await page.evaluate(async()=>{
      const {cardGlintFace}=await import('/src/lib/card-glint/card.mjs');
      const {renderCardGlint}=await import('/src/lib/card-glint/session.mjs');
      const cards=[
        {name:'Custom Wizard',type_line:'Creature — Wizard',colors:['U'],power:'2',toughness:'3',oracle_text:'Flying'},
        {name:'Custom Champion',type_line:'Legendary Creature — Warrior',colors:['R','W'],power:'3',toughness:'3',oracle_text:'Vigilance'},
        {name:'Custom Golem',card_types:['artifact','creature'],colors:[],power_toughness:'4/4'},
        {name:'Custom Grove',card_types:['land'],colors:[],color_identity:['G'],oracle_text:'{T}: Add {G}.'},
        {name:'Custom Walker',type_line:'Legendary Planeswalker — Test',colors:['U'],loyalty:3,oracle_text:'+1: Draw a card.\n−2: Tap target creature.'},
        {name:'Custom History',type_line:'Enchantment — Saga',colors:['W'],oracle_text:'I — Draw a card.\nII — Gain 2 life.\nIII — Create a token.'},
        {name:'Custom Siege',type_line:'Battle — Siege',colors:['R'],defense:4,oracle_text:'When this enters, draw a card.'},
      ];
      document.body.replaceChildren();document.body.style.cssText='display:flex;flex-wrap:wrap;gap:12px;padding:12px;background:#18202d';
      const results=[];
      for(const card of cards){
        const face=cardGlintFace(card,{printing:null});const {canvas}=await renderCardGlint(face);
        const copy=document.createElement('canvas');copy.width=canvas.width;copy.height=canvas.height;
        copy.getContext('2d').drawImage(canvas,0,0);copy.style.width='230px';copy.style.height='320px';document.body.append(copy);
        const bytes=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
        let opaque=0;for(let i=3;i<bytes.length;i+=4)if(bytes[i])opaque++;
        results.push({name:card.name,opaque:opaque/(canvas.width*canvas.height),style:face.frameStyle});
      }
      return results;
    });
    assert.equal(rendered.length,7);
    for(const card of rendered){assert.equal(card.style,'modern');assert.ok(card.opaque>.8,`${card.name} has a painted frame`);}
    await page.screenshot({path:'/tmp/ironsmith-default-glint-frames.png'});
  } finally {await browser.close();await vite.close();}
});

test('cached double-faced printing stays ready in the live preview', {timeout:30000}, async () => {
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();const browser=await chromium.launch();
  const id='12345678-1234-1234-1234-123456789abc';
  const art=side=>`https://cards.scryfall.io/normal/${side}/1/2/${id}.jpg`;
  const front={name:'Tamiyo, Inquisitive Student',type_line:'Legendary Creature — Moonfolk Wizard',mana_cost:'{U}',colors:['U'],power:'0',toughness:'3',oracle_text:'Flying\nWhenever Tamiyo attacks, investigate.',image_uris:{normal:art('front')}};
  const back={name:'Tamiyo, Seasoned Scholar',type_line:'Legendary Planeswalker — Tamiyo',colors:['U','G'],loyalty:'2',oracle_text:'+2: Draw a card.',image_uris:{normal:art('back')}};
  try {
    const page=await browser.newPage();page.setDefaultTimeout(8000);
    await page.route(`https://api.scryfall.com/cards/${id}`,route=>route.fulfill({json:{id,name:`${front.name} // ${back.name}`,layout:'transform',frame:'2015',card_faces:[front,back]}}));
    await page.route('https://cards.scryfall.io/**',route=>route.fulfill({contentType:'image/svg+xml',body:'<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect width="40" height="40" fill="teal"/></svg>'}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.evaluate(async url=>{const {resolveScryfallPrintingMetadata}=await import('/src/lib/scryfall.js');await resolveScryfallPrintingMetadata(url);},art('front'));
    await page.evaluate(({front,url})=>{window.changeGlintCard(front);window.setGlintArt(url);window.showGlintFrame();},{front,url:art('front')});
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:8000});
    await page.getByRole('button',{name:/Tamiyo, Inquisitive Student:/}).first().press('Enter');
    assert.equal(await page.evaluate(()=>window.glintActivated),'draw');
    await page.evaluate(({back,url})=>{window.changeGlintCard(back);window.setGlintArt(url);},{back,url:art('back')});
    await page.getByRole('img',{name:new RegExp(back.name)}).waitFor();
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:8000});
    await page.getByRole('button',{name:/Tamiyo, Seasoned Scholar:/}).first().press('Enter');
  } finally {await browser.close();await vite.close();}
});
