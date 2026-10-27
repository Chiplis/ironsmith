import {createServer} from 'vite';
import {chromium} from 'playwright';
import {readFile,mkdir} from 'node:fs/promises';
import assert from 'node:assert/strict';
const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
const browser=await chromium.launch();
try{
 const data=JSON.parse(await readFile('public/cards/yawgmoth-thran-physician.json','utf8'));
 const page=await browser.newPage({viewport:{width:1440,height:930},deviceScaleFactor:1});
 const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.addInitScript(art=>window.__abilityArt=art,data.scryfall.image_uris.normal);
 await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/ability-affordance-preview.html`,{waitUntil:'domcontentloaded'});
 await page.waitForFunction(()=>document.querySelectorAll('[data-card-glint-frame="ready"]').length===3,{},{timeout:120000});
 await page.waitForFunction(()=>document.querySelectorAll('.parchment-action').length===6);
 await page.evaluate(()=>document.fonts.ready);
 await page.locator('.ability-study').first().getByRole('button',{name:/Choose ability: Pay 1 life/}).click();
 assert.equal(await page.locator('.ability-study').first().locator('[data-selected="true"]').count(),1);
 await page.locator('.ability-study').first().getByRole('button',{name:/Cancel/}).click();
 await page.mouse.move(1400,910);
 await page.waitForTimeout(500);
 await mkdir('test-results',{recursive:true});
 await page.screenshot({path:'test-results/ability-affordance-concept.png',fullPage:true});
 console.log(JSON.stringify({ready:await page.locator('[data-card-glint-frame="ready"]').count(),buttons:await page.locator('.parchment-action').count(),errors}));
 assert.deepEqual(errors,[]);
}finally{await browser.close();await vite.close();}
