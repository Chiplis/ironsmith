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
