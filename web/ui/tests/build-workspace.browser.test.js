import {readFile} from 'node:fs/promises';
import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('Build searches into the editor and adds catalog cards or edited drafts', {timeout:60000}, async () => {
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  // Renderer coverage lives in card-glint-build; keep editor tests independent of GPU startup.
  await page.route('**/card-glint/*/magic.css',r=>r.abort());
  await page.route('https://**',r=>r.abort());
  await page.route(/\/cards\/[^/]+\.json(?:\?|$)/,r=>r.fulfill({status:404,body:''}));
  await page.route('**/cards/yavimaya-cradle-of-growth.json*',async r=>r.fulfill({json:JSON.parse(await readFile(new URL('../public/cards/yavimaya-cradle-of-growth.json',import.meta.url)))}));
  await page.route('**/cards/lightning-bolt.json*',async r=>r.fulfill({json:JSON.parse(await readFile(new URL('../public/cards/lightning-bolt.json',import.meta.url)))}));
  await page.route('https://api.scryfall.com/**',r=>r.fulfill({json:{name:'Lightning Bolt',type_line:'Instant',mana_cost:'{R}',oracle_text:'Lightning Bolt deals 3 damage to any target.',layout:'normal',games:['paper'],border_color:'black',set_type:'expansion'}}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`,{waitUntil:'domcontentloaded'});
  await page.getByRole('button',{name:'Build',exact:true}).click();
  const dialog=page.getByRole('dialog');
  assert.equal(await dialog.getByText('Forge',{exact:true}).count(),0);
  assert.equal(await dialog.getByRole('tab',{name:'Search cards'}).count(),0);
  assert.equal(await dialog.getByText('Placement',{exact:true}).count(),0);
  const footer=dialog.locator('.build-card-toolbar-actions');
  await footer.getByRole('combobox',{name:'Zone',exact:true}).selectOption('hand');
  assert.equal(await footer.getByRole('combobox',{name:'Player',exact:true}).count(),1);
  const searchBounds=await dialog.getByPlaceholder('Find a card to add or edit').boundingBox();
  const addBounds=await footer.getByRole('button',{name:'Add',exact:true}).boundingBox();
  assert.ok(Math.abs((searchBounds.y+searchBounds.height)-(addBounds.y+addBounds.height))<2,'Add aligns with the search field');
  await dialog.getByPlaceholder('Find a card to add or edit').fill('Yavimaya');
  await dialog.getByRole('button',{name:'Yavimaya, Cradle of Growth',exact:true}).click();
  await page.waitForFunction(()=>document.querySelector('.card-forge-inline-name')?.value==='Yavimaya, Cradle of Growth');
  await page.waitForTimeout(300);
  assert.equal(await dialog.getByRole('textbox',{name:'Card name',exact:true}).inputValue(),'Yavimaya, Cradle of Growth');
  assert.equal(await dialog.getByRole('button',{name:'Land',exact:true}).getAttribute('aria-pressed'),'true');
  assert.equal(await dialog.locator('.card-forge-status--error').count(),0);
  await dialog.getByRole('button',{name:'Add',exact:true}).click();
  assert.deepEqual(await page.evaluate(()=>window.lastAdded),{catalog:[0,'Yavimaya, Cradle of Growth','hand',false]});
  await page.getByRole('button',{name:'Build',exact:true}).click();
  await dialog.getByPlaceholder('Find a card to add or edit').fill('Lightning');
  await dialog.getByRole('button',{name:'Lightning Bolt',exact:true}).click();
  await dialog.getByRole('button',{name:'Add',exact:true}).waitFor();
  await page.screenshot({path:'test-results/build-workspace.png'});
  await dialog.getByRole('button',{name:'Add',exact:true}).click();
  assert.deepEqual(await page.evaluate(()=>window.lastAdded),{catalog:[0,'Lightning Bolt','hand',false]});
  await page.getByRole('button',{name:'Build',exact:true}).click();
  await dialog.getByRole('textbox',{name:'Card name',exact:true}).fill('Edited Bolt');
  await dialog.getByRole('button',{name:'Add',exact:true}).click();
  assert.equal((await page.evaluate(()=>window.lastAdded)).custom.draft.faces[0].name,'Edited Bolt');
  assert.equal((await page.evaluate(()=>window.lastAdded)).custom.zoneName,'hand');
 } finally {await browser.close();await vite.close();}
});

test('Build edits and adds baked cards while art is stalled, and ignores stale selections', {timeout: 60000}, async () => {
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
 await vite.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  const sourceRequests=[];
  const artRequests=[];
  await page.route('**/card-glint/*/magic.css',route=>{artRequests.push(route.request().url());});
  await page.route(/\/cards\/[^/]+\.json(?:\?|$)/,route=>{sourceRequests.push(route.request().url());return route.abort();});
  // Leave all third-party art, printing, font and frame requests pending.
  await page.route('https://**',route=>{artRequests.push(route.request().url());});
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`,{waitUntil:'domcontentloaded'});
  const payload=JSON.parse(await readFile(new URL('../public/cards/yavimaya-cradle-of-growth.json',import.meta.url)));
  await page.evaluate(async payload=>{
   const {installEmbeddedCardCatalog}=await import('/src/lib/embedded-card-catalog.js');
   window.holdBuildSource=false;
   installEmbeddedCardCatalog({ready:Promise.resolve(),getIndexJson:()=>({cards:[]}),getSourceJson:()=>{
    if(window.holdBuildSource)return new Promise(resolve=>{window.releaseBuildSource=()=>resolve(payload);});
    return payload;
   }});
  },payload);
  await page.getByRole('button',{name:'Build',exact:true}).click();
  const dialog=page.getByRole('dialog');
  const search=dialog.getByPlaceholder('Find a card to add or edit');
  const name=dialog.getByRole('textbox',{name:'Card name',exact:true});
  await search.fill('Yavimaya');
  await dialog.getByRole('button',{name:'Yavimaya, Cradle of Growth',exact:true}).click();
  await page.waitForFunction(()=>document.querySelector('.card-forge-inline-name')?.value==='Yavimaya, Cradle of Growth');
  await name.fill('Edited while art loads');
  await search.fill('Light');
  assert.equal(await name.inputValue(),'Edited while art loads');
  await dialog.getByRole('button',{name:'Add',exact:true}).click();
  assert.equal((await page.evaluate(()=>window.lastAdded)).custom.draft.faces[0].name,'Edited while art loads');
  assert.deepEqual(sourceRequests,[],'baked card rules never fetch card JSON');
  assert.ok(artRequests.length,'art dependencies remain pending during editing and adding');

  await page.getByRole('button',{name:'Build',exact:true}).click();
  await page.evaluate(()=>{window.holdBuildSource=true;});
  await search.fill('Yavimaya');
  await dialog.getByRole('button',{name:'Yavimaya, Cradle of Growth',exact:true}).click();
  await page.waitForFunction(()=>typeof window.releaseBuildSource==='function');
  await search.fill('Lightning');
  assert.equal(await search.inputValue(),'Lightning','search remains editable during a source lookup');
  await name.fill('Keep my newer edit');
  await page.evaluate(()=>window.releaseBuildSource());
  await page.waitForTimeout(250);
  assert.equal(await name.inputValue(),'Keep my newer edit','late selection cannot overwrite edits');
 } finally {await browser.close();await vite.close();}
});
