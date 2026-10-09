import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('live Fiery Islet displays blue/red mana while Arid Mesa has no mana strip', {timeout:90000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch({args:['--disable-webgl']});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  // The strip must work from engine snapshots, without external card metadata.
  await page.route('https://api.scryfall.com/**',r=>r.fulfill({status:404,body:''}));
  const puzzle={version:1,players:[{name:'Alice',life:20,zones:{battlefield:['Fiery Islet','Fiery Islet','Arid Mesa','Forest'],library:['Forest']}},{name:'Bob',life:20,zones:{library:['Mountain']}}]};
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?puzzle=${Buffer.from(JSON.stringify(puzzle)).toString('base64url')}`,{waitUntil:'domcontentloaded'});
  const land=name=>page.locator('.battlefield-row[data-bf-side="bottom"] .arena-permanent').filter({has:page.locator(`.arena-permanent-name[title="${name}"]`)}).first();
  await land('Fiery Islet').waitFor({timeout:60000});
  await land('Fiery Islet').locator('.arena-land-mana-strip').waitFor();
  assert.deepEqual(await land('Fiery Islet').locator('.arena-land-mana-strip > span').evaluateAll(els=>els.map(el=>el.dataset.manaColor)),['U','R']);
  assert.equal(await land('Fiery Islet').locator('.arena-land-mana-strip img, .arena-land-mana-strip svg').count(),0);
  assert.equal(await land('Arid Mesa').locator('.arena-land-mana-strip').count(),0);
  assert.equal(await land('Forest').locator('.arena-land-mana-strip > [data-mana-color="G"]').count(),1);
  const artSizes = await land('Fiery Islet').evaluate(face => {
    const card = face.closest('.game-card');
    const originalArt = parseFloat(getComputedStyle(face.querySelector('.arena-permanent-art')).height);
    const originalHeight = parseFloat(getComputedStyle(card).height);
    const clone = card.cloneNode(true);
    clone.querySelector('.arena-land-mana-strip').remove();
    clone.querySelector('.arena-permanent').removeAttribute('data-mana-strip');
    card.parentElement.append(clone);
    const plainArt = parseFloat(getComputedStyle(clone.querySelector('.arena-permanent-art')).height);
    const plainHeight = parseFloat(getComputedStyle(clone).height);
    clone.remove();
    return {originalArt,plainArt,originalHeight,plainHeight};
  });
  assert.ok(Math.abs(artSizes.originalArt-artSizes.plainArt)<1, JSON.stringify(artSizes));
  assert.ok(artSizes.originalHeight>artSizes.plainHeight, 'mana strip extends the frame below the original artwork');
  await page.screenshot({path:'/tmp/live-land-mana-strips.png'});
 }finally{await browser.close();await vite.close();}
});
