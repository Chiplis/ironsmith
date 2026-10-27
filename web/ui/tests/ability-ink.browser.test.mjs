import test from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
import {chromium} from 'playwright';
import {readFile} from 'node:fs/promises';
test('renderer uses blue, green and red ink and blocks unavailable activations',{timeout:120000},async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];
  page.on('pageerror',error=>errors.push(error.message));
  const data=JSON.parse(await readFile('public/cards/yawgmoth-thran-physician.json','utf8'));
  await page.addInitScript(art=>window.__abilityArt=art,data.scryfall.image_uris.normal);
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/ability-ink-preview.html`,{waitUntil:'domcontentloaded'});
  await page.waitForFunction(()=>document.querySelectorAll('[data-card-glint-frame="ready"]').length===3);
  const available=page.locator('[data-example=available] .mse-card-hit-target').first();
  const unavailable=page.locator('[data-example=unavailable] .mse-card-hit-target').first();
  await available.hover();assert.equal(await available.getAttribute('data-ability-state'),'available');
  await unavailable.hover();
  assert.equal(await unavailable.getAttribute('data-ability-state'),'unavailable');
  await unavailable.evaluate(node=>node.click());assert.deepEqual(await page.evaluate(()=>window.inkActivations),[]);
  assert.match(await unavailable.getAttribute('title'),/Cannot pay/);
  // Keep both hover treatments visible for the comparison screenshot.
  await available.focus();
  await page.evaluate(()=>document.fonts.ready);
  await page.screenshot({path:'test-results/ability-ink-states.png',fullPage:true});
  const painted=await page.locator('.ability-text-ink').evaluateAll(canvases=>canvases.map(canvas=>{
    const pixels=canvas.getContext('2d').getImageData(0,0,canvas.width,canvas.height).data;
    let count=0;for(let i=3;i<pixels.length;i+=4)if(pixels[i]>100)count++;return count;
  }));
  assert.ok(painted.every(count=>count>500),JSON.stringify(painted));
  // The first symbol in the highlighted paragraph is {T}. Check its pixels,
  // not just the text, so omitting the symbol painter cannot regress silently.
  const tapPixels=await page.locator('.ability-study').evaluateAll(studies=>studies.map(study=>{
    const canvas=study.querySelector('.ability-text-ink');
    const target=study.querySelector('.mse-card-hit-target');
    const bounds=canvas.getBoundingClientRect(), hit=target.getBoundingClientRect();
    const scale=canvas.width/bounds.width;
    const pixels=canvas.getContext('2d').getImageData(
      Math.round((hit.left-bounds.left+5)*scale), Math.round((hit.top-bounds.top+3)*scale),
      Math.round(15*scale), Math.round(17*scale),
    ).data;
    let count=0;for(let i=0;i<pixels.length;i+=4)if(pixels[i+3]>100)count++;
    return count;
  }));
  assert.ok(tapPixels.every(count=>count>10),`Tap arrows must receive state ink: ${tapPixels}`);
  await available.click();assert.deepEqual(await page.evaluate(()=>window.inkActivations),[1]);
  assert.equal(await available.getAttribute('data-ability-state'),'selected');
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await vite.close();}
});
