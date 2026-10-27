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
    const fallback=page.locator('[data-frame-mode="baked"]');
    await fallback.waitFor();
    await fallback.locator('.simple-card-frame__stock').evaluate(image=>image.decode());
    const artBox=await fallback.locator('.simple-card-frame__art').boundingBox();
    const loaderBox=await fallback.locator('[data-card-art-loader]').boundingBox();
    assert.ok(loaderBox.x>=artBox.x-1 && loaderBox.y>=artBox.y-1);
    assert.ok(loaderBox.x+loaderBox.width<=artBox.x+artBox.width+1);
    assert.ok(loaderBox.y+loaderBox.height<=artBox.y+artBox.height+1);
    assert.equal(await fallback.locator('[data-mana-color="U"]').count(),1);
    await page.screenshot({path:'test-results/card-glint-baked-fallback.png'});

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
    await page.waitForFunction(()=>window.glintReadyAtFirstPaint!==undefined);
    assert.equal(await page.evaluate(()=>window.glintReadyAtFirstPaint),true,'completed background frame is visible on the first paint');
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

test('custom Island uses a default glint frame with local art and source attribution', {timeout:30000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    await page.route('https://api.scryfall.com/**',route=>route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.evaluate(async()=>{
      const {customCardAssets,customCardImageUrl}=await import('/src/lib/custom-card-assets.js');
      const island=customCardAssets('Island');
      window.changeGlintCard({name:'Island',type_line:island.typeLine,oracle_text:'',color_identity:['U'],flavor_text:island.flavorText});
      window.setGlintArt(customCardImageUrl('Island'));
      window.setGlintRules({lines:[],actions:new Map()});window.showGlintFrame();
    });
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:15000});
    assert.equal(await page.locator('[data-card-glint-frame="error"]').count(),0);
    const source=page.getByRole('link',{name:'Open original post on reddit.com'});
    const href='https://www.reddit.com/r/custommagic/comments/ch910l/bob_ross_basic_lands/';
    assert.equal(await source.getAttribute('href'),href);
    const linkBox=await source.boundingBox(),frameBox=await page.locator('[data-card-glint-frame="ready"]').boundingBox();
    assert.ok(linkBox.y>=frameBox.y+frameBox.height*.93,'attribution stays in the bottom black border');
    assert.ok(linkBox.y+linkBox.height<frameBox.y+frameBox.height,'attribution stays inside the card');
    assert.ok(linkBox.x>=frameBox.x && linkBox.x+linkBox.width<=frameBox.x+frameBox.width,'attribution fits horizontally');
    await page.locator('.card-frame-presentation').screenshot({path:'/tmp/custom-island-source-link.png'});
    await page.context().route('https://www.reddit.com/**',route=>route.fulfill({contentType:'text/html',body:'Original post'}));
    const popupPromise=page.waitForEvent('popup');
    await source.click();
    const popup=await popupPromise;
    await popup.waitForLoadState();
    assert.equal(popup.url(),href);
    await popup.close();
  }finally{await browser.close();await vite.close();}
});

test('land Sagas select plates from live intrinsic mana and revert when it is removed',async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    const variants=await page.evaluate(async()=>{
      const {loadCardGlint}=await import('/src/lib/card-glint/session.mjs');
      const {cardGlintFace}=await import('/src/lib/card-glint/card.mjs');
      const {cardFrameSnapshot}=await import('/src/lib/card-frame-snapshot.mjs');
      const api=await loadCardGlint();
      const card={name:"Urza's Saga",type_line:'Enchantment Land — Saga',colors:[],color_identity:[],oracle_text:'I — Add {C}.',layout:'saga'};
      return ['', ' Forest', ' Island Forest', ''].map(extra=>{
        const face=cardGlintFace(JSON.parse(cardFrameSnapshot({...card,type_line:card.type_line+extra})));
        return {colors:face.colors,palette:face.frameColors,assets:api.canvasAssetUrls(face).filter(Boolean)};
      });
    });
    assert.deepEqual(variants.map(v=>v.palette),[[],['G'],['U','G'],[]]);
    for(const v of variants)assert.deepEqual(v.colors,[],'frame tint does not change game colors');
    assert.ok(variants[0].assets.some(url=>url.endsWith('/saga/regular/l.png')));
    assert.ok(variants[1].assets.some(url=>url.endsWith('/sagaFrameG.png')));
    assert.ok(!variants[1].assets.some(url=>url.endsWith('/saga/regular/l.png')));
    for(const color of ['U','G'])assert.ok(variants[2].assets.some(url=>url.endsWith(`/sagaFrame${color}.png`)));
    assert.deepEqual(variants[3],variants[0]);
  }finally{await browser.close();await vite.close();}
});

test('live land Saga preview retains the glint frame and granted mana actions', {timeout:60000},async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.evaluate(()=>{
      const lines=[
        'I — This Saga gains "{T}: Add {C}."',
        'II — This Saga gains "{2}, {T}: Create a 0/0 colorless Construct artifact creature token with \'This token gets +1/+1 for each artifact you control.\'"',
        'III — Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.',
        '{T}: Add {C}.',
      ];
      window.changeGlintCard({name:"Urza's Saga",type_line:'Enchantment Land — Urzas Saga Forest',colors:[],color_identity:[],oracle_text:lines.join('\n')});
      window.setGlintRules({lines,actions:new Map([[2,[{id:'chapter'}]],[3,[{id:'mana'}]]])});window.showGlintFrame();
    });
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:40000});
    await page.getByRole('button',{name:/Urza's Saga: III/}).first().press('Enter');
    assert.equal(await page.evaluate(()=>window.glintActivated),'chapter');
    await page.getByRole('button',{name:"Urza's Saga: {T}: Add {C}.",exact:true}).first().press('Enter');
    assert.equal(await page.evaluate(()=>window.glintActivated),'mana');
  }finally{await browser.close();await vite.close();}
});

test('colored lands paint their outer stock from the land palette', {timeout:120000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1100,height:1100}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    const results=await page.evaluate(async()=>{
      const {cardGlintFace}=await import('/src/lib/card-glint/card.mjs');
      const {renderCardGlint,loadCardGlint}=await import('/src/lib/card-glint/session.mjs');
      const api=await loadCardGlint();
      document.body.replaceChildren();document.body.style.cssText='display:flex;flex-wrap:wrap;gap:10px;background:#222';
      const cards=[
        ['Yavimaya, Cradle of Growth','Legendary Land — Forest',['G']],
        ['Island','Basic Land — Island',['U']],
        ['Plains','Basic Land — Plains',['W']],
        ['Swamp','Basic Land — Swamp',['B']],
        ['Mountain','Basic Land — Mountain',['R']],
        ['Forest','Basic Land — Forest',['G']],
        ['Dual land','Land',['U','G']],
        ['Wastes','Basic Land',[]],
        ['Forest without identity','Basic Land — Forest',undefined],
      ];
      const results=[];
      for(const [name,type_line,palette] of cards){
        const face=cardGlintFace({name,type_line,colors:[],color_identity:palette});
        const {canvas}=await renderCardGlint(face);
        const copy=document.createElement('canvas');copy.width=canvas.width;copy.height=canvas.height;
        copy.getContext('2d').drawImage(canvas,0,0);copy.style.width='244px';copy.style.height='340px';document.body.append(copy);
        const pixels=canvas.getContext('2d').getImageData(Math.round(canvas.width*.045),Math.round(canvas.height*.3),1,1).data;
        results.push({name,colors:face.colors,assets:api.canvasAssetUrls(face),pixel:[...pixels]});
      }
      return results;
    });
    for(const [index,color] of ['G','U','W','B','R','G','Gold','Land','G'].entries()){
      assert.deepEqual(results[index].colors,[]);
      assert.ok(results[index].assets.some(url=>url?.endsWith(`/bg/${color}.webp`) || (color==='Land' && url?.endsWith('/m15/new/l.png'))),JSON.stringify(results[index]));
    }
    const [r,g,b]=results[0].pixel;
    assert.ok(g>r && g>b,`Yavimaya outer stock is green: ${results[0].pixel}`);
    const [ir,ig,ib]=results[1].pixel;
    assert.ok(ib>ir && ib>ig,`Island outer stock is blue: ${results[1].pixel}`);
    const [fr,fg,fb]=results[8].pixel;
    assert.ok(fg>fr && fg>fb,`Forest without identity has green outer stock: ${results[8].pixel}`);
    await page.screenshot({path:'test-results/card-glint-land-stock.png'});
  } finally {await browser.close();await vite.close();}
});

test('Kaito keeps a painted frame and loyalty actions while he is a creature', {timeout:90000},async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:440,height:570}}),errors=[];
    page.on('pageerror',error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.waitForFunction(()=>window.changeGlintCard);
    // Preserve the verbose live engine wording that overflowed the ordinary
    // creature frame; no Scryfall lookup or replacement Oracle text is needed.
    await page.evaluate(()=>{
      const lines=['Ninjutsu {1}{U}{B}',
        'As long as timing restriction: During your turn and this has one or more loyalty counters on him, kaito, Bane of Nightmares is a creature.',
        'As long as this has one or more loyalty counters on him, Kaito is ninja during your turn and and has base power and toughness 3/4 during your turn and.',
        'Kaito has hexproof as long as timing restriction: During your turn and this has one or more loyalty counters on him.',
        '+1: You get an emblem with "Ninjas you control get +1/+1."',
        '0: Surveil 2. Then you draw a card for each opponent who has lost life this turn.',
        '−2: Tap target creature. Put two stun counters on it.','Hexproof','4 loyalty'];
      window.changeGlintCard({name:'Kaito, Bane of Nightmares',type_line:'Legendary Creature — Ninja',
        mana_cost:'{2}{U}{B}',power:'3',toughness:'4',loyalty:4,oracle_text:lines.join('\n')});
      window.setGlintRules({lines,actions:new Map([[4,[{id:'plus'}]],[5,[{id:'zero'}]],[6,[{id:'minus'}]]])});
      window.showGlintFrame();
    });
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:60000});
    await page.getByRole('img',{name:/Kaito, Bane of Nightmares: Legendary Creature — Ninja/}).waitFor();
    await page.locator('.card-frame-fallback').waitFor({state:'detached'});
    assert.equal(await page.locator('[data-frame-mode="baked"]').count(),0);
    for(const [prefix,id] of [['+1:','plus'],['0:','zero'],['−2:','minus']]){
      await page.getByRole('button',{name:`Kaito, Bane of Nightmares: ${prefix}`,exact:false}).first().click();
      assert.equal(await page.evaluate(()=>window.glintActivated),id);
    }
    await page.screenshot({path:'/tmp/kaito-card-glint-fixed.png'});
    assert.deepEqual(errors,[]);
  }finally{await browser.close();await vite.close();}
});

test('Adventure and Aftermath paint dedicated frames', {timeout:90000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    const results=await page.evaluate(async()=>{
      const {cardGlintFace}=await import('/src/lib/card-glint/card.mjs');
      const {renderCardGlint,loadCardGlint}=await import('/src/lib/card-glint/session.mjs');
      const api=await loadCardGlint(),results=[];
      document.body.replaceChildren();
      for(const layout of ['adventure','aftermath','split']){
        const face=cardGlintFace({layout,card_faces:[
          {name:'Guardian',type_line:layout==='adventure'?'Creature — Human':'Sorcery',mana_cost:'{2}{G}',oracle_text:'Draw a card.',power:'3',toughness:'3'},
          {name:'Journey',type_line:layout==='adventure'?'Instant — Adventure':'Sorcery',mana_cost:'{U}',oracle_text:layout==='adventure'?'Draw a card.':'Aftermath\nDraw a card.'},
        ]});
        const {canvas}=await renderCardGlint(face);
        const copy=document.createElement('canvas');copy.width=canvas.width;copy.height=canvas.height;
        copy.getContext('2d').drawImage(canvas,0,0);copy.style.width='244px';document.body.append(copy);
        results.push({layout:face.layout,assets:api.canvasAssetUrls(face),width:canvas.width});
      }
      return results;
    });
    assert.deepEqual(results.map(r=>r.layout),['adventure','aftermath','aftermath']);
    for(const result of results){
      assert.ok(result.width>0);
      assert.ok(result.assets.some(url=>url?.includes(result.layout==='adventure'?'/adventure/':'/aftermath/')));
    }
    await page.screenshot({path:'/tmp/card-glint-adventure-aftermath.png'});
  }finally{await browser.close();await vite.close();}
});

test('imported Prepare Room Flip modal DFC Aftermath and Battle frames paint', {timeout:120000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1500,height:1100}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    const results=await page.evaluate(async()=>{
      const {cardBuildDraft}=await import('/src/lib/card-build-draft.js');
      const {cardGlintDraftFace}=await import('/src/lib/card-glint/draft.mjs');
      const {specialLayoutCard}=await import('/tests/card-glint-special-layouts-fixture.mjs');
      const {renderCardGlint}=await import('/src/lib/card-glint/session.mjs');
      document.body.replaceChildren();document.body.style.cssText='display:flex;flex-wrap:wrap;align-content:flex-start;gap:10px;background:#222';
      const results=[];
      for(const layout of ['prepare','room','flip','modal_dfc','aftermath','battle']) {
        const draft=cardBuildDraft(specialLayoutCard(layout));
        for(const index of ['modal_dfc','battle'].includes(layout)?[0,1]:[0]) {
          const input=cardGlintDraftFace(draft,index);
          const {canvas,face,api}=await renderCardGlint(input);
          const wrapper=document.createElement('div');wrapper.style.color='white';
          const label=document.createElement('div');label.textContent=`${layout} / ${index}`;wrapper.append(label);
          const copy=document.createElement('canvas');copy.width=canvas.width;copy.height=canvas.height;
          copy.getContext('2d').drawImage(canvas,0,0);copy.style.width='244px';wrapper.append(copy);document.body.append(wrapper);
          results.push({layout,index,name:face.name,panels:face.faces?.length??0,template:face.frameTemplate,reverseStats:face.reverseStats,assets:api.canvasAssetUrls(face),width:canvas.width});
        }
      }
      return results;
    });
    assert.equal(results.length,8);
    for(const r of results){
      assert.ok(r.width>0);
      if(['prepare','room','flip','aftermath'].includes(r.layout))assert.equal(r.panels,2);
      if(r.layout==='prepare')assert.ok(r.assets.some(url=>url?.includes('/prepare/')));
      if(r.layout==='aftermath')assert.ok(r.assets.some(url=>url?.includes('/aftermath/')));
      if(r.layout==='flip')assert.equal(r.template,'flip');
      if(r.index===1)assert.equal(r.name,'Journey');
      if(r.layout==='battle'&&r.index===0){assert.equal(r.template,'battle');assert.deepEqual(r.reverseStats,{power:'5',toughness:'6'});}
    }
    await page.screenshot({path:'/tmp/card-glint-special-layouts.png',fullPage:true});
  }finally{await browser.close();await vite.close();}
});

test('remaining frame families render through the host adapter', {timeout:120000},async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});await vite.listen();
  const browser=await chromium.launch();
  try{
    const page=await browser.newPage({viewport:{width:1500,height:1100}});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    const results=await page.evaluate(async()=>{
      const {frameAuditCases}=await import('/tests/card-glint-frame-audit-fixture.mjs');
      const {cardGlintFace}=await import('/src/lib/card-glint/card.mjs');
      const {renderCardGlint}=await import('/src/lib/card-glint/session.mjs');
      document.body.replaceChildren();document.body.style.cssText='display:flex;flex-wrap:wrap;align-content:flex-start;gap:10px;background:#222';
      const results=[];
      for(const {id,card,kind} of frameAuditCases()){
        const {canvas,face,api}=await renderCardGlint(cardGlintFace(card));
        const box=document.createElement('div');box.style.color='white';
        const label=document.createElement('div');label.textContent=id;box.append(label);
        const copy=document.createElement('canvas');copy.width=canvas.width;copy.height=canvas.height;
        copy.getContext('2d').drawImage(canvas,0,0);copy.style.width='244px';box.append(copy);document.body.append(box);
        results.push({id,expected:kind,kind:api.faceFrame(face),assets:api.canvasAssetUrls(face),family:face.frameAssetFamily,treatment:face.frameTreatment,stats:face.reverseStats});
      }
      return results;
    });
    for(const r of results)assert.equal(r.kind,r.expected,r.id);
    assert.equal(results.find(r=>r.id==='eighth').family,'eighth');
    assert.equal(results.find(r=>r.id==='colorshifted').treatment,'colorshifted');
    assert.deepEqual(results.find(r=>r.id==='transforming-saga').stats,{power:'4',toughness:'5'});
    assert.ok(results.find(r=>r.id==='fuse').assets.some(url=>url?.includes('/fuse/')));
    await page.screenshot({path:'/tmp/card-glint-frame-audit.png',fullPage:true});
  }finally{await browser.close();await vite.close();}
});
