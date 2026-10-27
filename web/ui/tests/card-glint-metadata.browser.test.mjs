import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('card-glint renders and activates while printing metadata is pending', {timeout:60000}, async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();
  const browser = await chromium.launch();
  let releaseMetadata;
  const gate = new Promise(resolve => { releaseMetadata = resolve; });
  try {
    const page = await browser.newPage();
    const id = '11111111-1111-1111-1111-111111111111';
    let requested = false;
    await page.route(`https://api.scryfall.com/cards/${id}`, async route => {
      requested = true;
      await gate;
      await route.fulfill({json:{id,frame:'1997',rarity:'rare',set:'tmp'}});
    });
    await page.route('https://cards.scryfall.io/**', route => route.fulfill({
      contentType:'image/svg+xml',body:'<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="blue"/></svg>',
    }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.waitForFunction(() => window.setGlintArt);
    await page.evaluate(id => {
      window.setGlintArt(`https://cards.scryfall.io/normal/front/1/1/${id}.jpg`);
      window.showGlintFrame();
    }, id);
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:45000});
    assert.ok(requested, 'metadata request is still pending');
    await page.getByRole('button',{name:'Glint Test Adept: {T}: Draw a card.'}).click();
    assert.equal(await page.evaluate(() => window.glintActivated), 'draw');
    const before = await page.locator('canvas').evaluate(canvas => canvas.toDataURL());
    await page.locator('.card-frame-fallback').waitFor({state:'detached'});
    await page.evaluate(() => {
      window.holdGlintUpgrade(true);
      window.frameRegressions=[];
      const surface=document.querySelector('.card-glint-surface');
      window.frameObserver=new MutationObserver(records=>{
        for(const record of records){
          if(record.type==='attributes' && record.target===surface && surface.style.display==='none')
            window.frameRegressions.push('hidden canvas');
          for(const node of record.addedNodes){
            if(node.nodeType===1 && (node.matches('.card-frame-fallback') || node.querySelector('.card-frame-fallback')))
              window.frameRegressions.push('remounted fallback');
          }
        }
      });
      window.frameObserver.observe(document.getElementById('root'),{subtree:true,childList:true,attributes:true,attributeFilter:['style']});
    });
    releaseMetadata();
    await page.waitForFunction(()=>window.isGlintPrintingReady());
    assert.equal(await page.locator('canvas').evaluate(canvas=>canvas.toDataURL()),before,'interaction holds the completed cosmetic upgrade');
    assert.deepEqual(await page.evaluate(()=>window.frameRegressions),[]);
    await page.evaluate(()=>window.holdGlintUpgrade(false));
    await page.waitForFunction(before => {
      const canvas = document.querySelector('[data-card-glint-frame="ready"]');
      return canvas && canvas.toDataURL() !== before;
    }, before);
    assert.deepEqual(await page.evaluate(()=>window.frameRegressions),[],'metadata never hides the existing frame or remounts its fallback');
    await page.evaluate(()=>{
      window.frameObserver.disconnect();
      window.holdGlintUpgrade(true);
      window.changeGlintCard(card=>({...card,oracle_text:'{T}: Gain 2 life.',power:'5'}));
    });
    await page.getByRole('img',{name:/Gain 2 life/}).waitFor();
    await page.getByRole('button',{name:'Glint Test Adept: {T}: Gain 2 life.'}).waitFor();
    assert.equal(await page.getByRole('button',{name:'Glint Test Adept: {T}: Draw a card.'}).count(),0,'old hit targets do not survive a rules update');
  } finally {
    releaseMetadata();
    await browser.close();
    await vite.close();
  }
});

test('cold cards keep their original fallback through interaction and release it with motion preferences', {timeout:60000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();const browser=await chromium.launch();
  try{
    for(const reducedMotion of ['no-preference','reduce']){
      const page=await browser.newPage({reducedMotion});
      let release;
      const gate=new Promise(resolve=>{release=resolve;});
      await page.route('**/card-glint/**/magic-presentation.js',async route=>{await gate;await route.continue();});
      try{
        await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html`);
        await page.locator('.card-frame-fallback').waitFor();
        await page.evaluate(()=>{
          window.holdGlintUpgrade(true);
          window.originalFallback=document.querySelector('.card-frame-fallback');
          window.fadeObservation=null;
          new MutationObserver(()=>{
            const node=document.querySelector('.card-frame-fallback[data-leaving=true]');
            if(node)window.fadeObservation={same:node===window.originalFallback,animation:getComputedStyle(node).animationName,inert:node.inert};
          }).observe(document.getElementById('root'),{subtree:true,attributes:true,childList:true});
        });
        release();
        await page.evaluate(()=>window.warmGlintFrame());
        assert.equal(await page.locator('[data-card-glint-frame="ready"]').count(),0,'initial cosmetic upgrade waits while held');
        assert.equal(await page.evaluate(()=>document.querySelector('.card-frame-fallback')===window.originalFallback),true);
        await page.evaluate(()=>window.holdGlintUpgrade(false));
        await page.locator('[data-card-glint-frame="ready"]').waitFor();
        await page.waitForFunction(()=>window.fadeObservation);
        const observed=await page.evaluate(()=>window.fadeObservation);
        assert.equal(observed.same,true,'fade reuses the fitted fallback');
        assert.equal(observed.inert,true,'outgoing fallback cannot intercept actions');
        assert.equal(observed.animation,reducedMotion==='reduce'?'none':'card-frame-fallback-out');
        await page.locator('.card-frame-fallback').waitFor({state:'detached'});
        await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
        await page.waitForFunction(()=>window.warmGlintFrame);
        await page.evaluate(()=>window.warmGlintFrame());
        await page.evaluate(()=>{window.holdGlintUpgrade(true);window.showGlintFrame();});
        await page.locator('[data-card-glint-frame="ready"]').waitFor();
        assert.equal(await page.locator('.card-frame-fallback').count(),0,'cached first paint bypasses the fallback even during interaction');
      }finally{release();await page.close();}
    }
  }finally{await browser.close();await vite.close();}
});
