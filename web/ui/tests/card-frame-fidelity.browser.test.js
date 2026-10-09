import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright';
import registrations from '../src/lib/card-region-history.generated.js';
const cache=process.env.FRAME_HISTORY_CACHE;
test('original wording fits fixed ability bands without losing the final line',{skip:!cache,timeout:180000},async()=>{
 const corpus=JSON.parse(await readFile(new URL('./frame-history-corpus.json',import.meta.url)));
 const prefixes=['9c60d186','ae6d5319','5fd218be','a0baccde','f5453591'];
 const picked=registrations.filter(r=>prefixes.some(prefix=>r.id.startsWith(prefix)));
 const vite=await createServer({root:fileURLToPath(new URL('../',import.meta.url)),server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch();
 try{for(const registration of picked){
  const c=corpus.cases.find(c=>c.id===registration.id&&c.face===registration.face),printing=JSON.parse(await readFile(join(cache,c.id+'.json'))),page=await browser.newPage();
  await page.addInitScript(data=>{window.__regionFixture=data;localStorage.setItem('ironsmith.locale','en');},{registration,printing,fidelity:true});
  await page.route('https://cards.scryfall.io/**',async route=>route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(cache,c.slug+'-normal.jpg'))}));
  await page.route('https://api.scryfall.com/**',async route=>route.fulfill({json:{}}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-live-regions.html`,{waitUntil:'domcontentloaded'});
  await page.waitForSelector('.registered-card-frame__scan[data-mask-ready="true"]');
  for(const width of [240,420]){
   await page.locator('[data-live-frame]').evaluate((node,width)=>{node.style.width=width+'px';node.style.height=width*680/488+'px';},width);
   await page.evaluate(async()=>{await document.fonts.ready;await Promise.all([...document.images].map(image=>image.decode()));for(let i=0;i<16;i++)await new Promise(requestAnimationFrame);});
   const clipped=await page.locator('[data-field-kind="rule"][data-replaced="true"]').evaluateAll((fields,rotation)=>fields.flatMap(field=>{
    const paragraph=field.querySelector('.interactive-card-frame__rule-line');if(!paragraph)return [];
    const rects=[],walker=document.createTreeWalker(paragraph,NodeFilter.SHOW_TEXT);
    while(walker.nextNode())for(const match of walker.currentNode.textContent.matchAll(/\S+/g)){
     const range=document.createRange();range.setStart(walker.currentNode,match.index);range.setEnd(walker.currentNode,match.index+match[0].length);rects.push(...range.getClientRects());
    }
    rects.push(...[...paragraph.querySelectorAll('img,svg')].map(node=>node.getBoundingClientRect()));
    const text={left:Math.min(...rects.map(r=>r.left)),right:Math.max(...rects.map(r=>r.right)),top:Math.min(...rects.map(r=>r.top)),bottom:Math.max(...rects.map(r=>r.bottom))},column=field.closest('.registered-card-frame__column'),bounds=(column||field).getBoundingClientRect();
    const transformed=getComputedStyle(field).transform!=='none';
    const clipped=rotation===90 ? text.right>bounds.right+1||text.top<bounds.top-1||text.bottom>bounds.bottom+1 : text.bottom>bounds.bottom+1||text.left<bounds.left-1||text.right>bounds.right+1;
    return !transformed&&clipped?[{text:field.dataset.liveText,bottom:text.bottom,limit:bounds.bottom,left:text.left,right:text.right,boundsLeft:bounds.left,boundsRight:bounds.right}]:[];
   }),registration.rotation||0);
   assert.deepEqual(clipped,[],`${c.name} at ${width}px`);
  }
  await page.close();
 }}finally{await browser.close();await vite.close();}
});
