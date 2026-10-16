import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
const decks=Array.from({length:24},(_,i)=>({id:`deck-${i}`,name:`Test deck ${i}`,format:'modern',mainboardCount:60,cardNames:['Mountain'],detail:`details/deck-${i}.json`}));
test('Play tabs fill the viewport, preserve featured decks while scrolling, and keep the selected deck beside the browser', {timeout:120000}, async()=>{
  const server=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await server.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
    await page.route('**/catalog/*/index.json',r=>r.fulfill({json:{schemaVersion:1,format:'modern',decks}}));
    await page.route('**/catalog/*/search-index.json',r=>r.fulfill({status:404,body:''}));
    await page.route('**/catalog/*/details/*.json',r=>r.fulfill({json:{id:'test',name:'Test deck',format:'modern',mainboard:[{name:'Mountain',count:60}],sideboard:[],commander:[]}}));
    await page.route('**/lobbies',r=>r.fulfill({json:{lobbies:[]}}));
    for(const viewport of [{width:1440,height:900},{width:1280,height:720}]) {
      await page.setViewportSize(viewport);
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/play-workspace.html`);
      for(const tab of ['Create','Join','Goldfish','Tournaments']) {
        await page.getByRole('button',{name:tab,exact:true}).first().click();
        const sheet=await page.locator('.play-workspace').boundingBox();
        assert.equal(Math.round(sheet.width),viewport.width);assert.equal(Math.round(sheet.height),viewport.height);
        if(tab==='Create'||tab==='Join') {
          await page.locator('[data-featured-deck]').first().waitFor();
          const featured=page.locator('[data-featured-decks]');const before=await featured.boundingBox();
          const list=page.locator('[data-deck-catalog-list]');await list.evaluate(e=>{e.scrollTop=500});
          assert.deepEqual(await featured.boundingBox(),before);
          const sidebar=await page.locator('.play-deck-sidebar').boundingBox();const catalog=await page.locator('[data-deck-catalog]').boundingBox();
          assert.ok(sidebar.x>=catalog.x+catalog.width);
          assert.ok(sidebar.y+sidebar.height<=viewport.height+1);
          assert.ok(before.y+before.height<=catalog.y+catalog.height);
          const clipping=await page.locator('.play-setup-grid, .play-session-controls, .play-deck-sidebar').evaluateAll(nodes=>nodes.filter(e=>e.scrollHeight>e.clientHeight+2).map(e=>({class:e.className,height:e.clientHeight,scroll:e.scrollHeight})));
          assert.deepEqual(clipping,[],`${tab} ${viewport.width}: clipped panels`);
          await featured.getByRole('button',{name:'Use',exact:true}).first().click();
          await page.waitForFunction(()=>document.querySelector('.play-deck-sidebar textarea')?.value.includes('60 Mountain'));
          assert.equal(await page.locator('.play-deck-sidebar textarea').first().inputValue(),'Deck\n60 Mountain');
        }
      }
    }
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await server.close();}
});
