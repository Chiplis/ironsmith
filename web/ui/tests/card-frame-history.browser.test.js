import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright';
import registrations from '../src/lib/card-region-history.generated.js';
const cache=process.env.FRAME_HISTORY_CACHE;
test('historical and segmented registrations fit changed rails and preserve pixels outside text',{skip:!cache,timeout:180000},async()=>{
 const corpus=JSON.parse(await readFile(new URL('./frame-history-corpus.json',import.meta.url)));
 const picked=['class','case','saga','adventure','prototype','modal_dfc'].map(layout=>registrations.find(r=>r.layout===layout));picked.push(registrations.find(r=>r.set==='mps'),registrations.find(r=>r.set==='mp2'&&r.fields.some(f=>f.stackedStats)),registrations.find(r=>r.fields.some(f=>f.rotation===90)),registrations.find(r=>r.fields.some(f=>f.protectedBounds?.length)));
 const vite=await createServer({root:fileURLToPath(new URL('../',import.meta.url)),server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
 try{for(const registration of picked){
  assert.ok(registration);
  const c=corpus.cases.find(c=>c.id===registration.id&&c.face===registration.face),printing=JSON.parse(await readFile(join(cache,c.id+'.json'))),page=await browser.newPage({viewport:{width:850,height:1100}});
  const rules=registration.fields.filter(f=>f.kind==='rule');
  await page.addInitScript(data=>{window.__regionFixture=data;localStorage.setItem('ironsmith.locale','en');},{registration,printing,liveRules:rules.map(f=>f.text+' Additional changed text.'+' Additional text.'.repeat(4))});
  await page.route('https://cards.scryfall.io/**',async r=>r.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(cache,c.slug+'-normal.jpg'))}));
  await page.route('https://api.scryfall.com/**',r=>r.fulfill({json:{}}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-live-regions.html`);
  await page.waitForFunction(count=>document.querySelectorAll('[data-field-kind="rule"][data-replaced="true"]').length===count,rules.length);
  for(const width of [240,740]){
   await page.locator('[data-live-frame]').evaluate((el,width)=>{el.style.width=width+'px';el.style.height=width*680/488+'px';},width);
   await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))));
   await page.waitForFunction(()=>[...document.querySelectorAll('[data-field-kind="name"][data-replaced="true"], [data-field-kind="type"][data-replaced="true"]')].every(n=>n.querySelector('[data-text-overflow="false"]')));
   const overflow=await page.locator('[data-field-kind="rule"] .interactive-card-frame__rules').evaluateAll(nodes=>nodes.map(n=>({horizontal:n.scrollWidth-n.clientWidth,vertical:n.scrollHeight-n.clientHeight,scroll:getComputedStyle(n).overflowY})));
   for(const box of overflow){assert.ok(box.horizontal<=2,JSON.stringify({name:c.name,width,box}));assert.ok(box.vertical<=2||box.scroll==='auto',JSON.stringify({name:c.name,width,box}));}
  }
  const changedOutside=await page.evaluate(async registration=>{
   const original=new Image();original.crossOrigin='anonymous';original.src=registration.source;await original.decode().catch(e=>{throw new Error(registration.id+" original: "+e.message)});
   const clean=document.querySelector('.registered-card-frame__scan');await clean.decode().catch(e=>{throw new Error(registration.id+" masked: "+e.message)});const canvas=document.createElement('canvas');canvas.width=original.width;canvas.height=original.height;const ctx=canvas.getContext('2d',{willReadFrequently:true});ctx.drawImage(original,0,0);const before=ctx.getImageData(0,0,canvas.width,canvas.height).data;ctx.drawImage(clean,0,0);const after=ctx.getImageData(0,0,canvas.width,canvas.height).data;
   const allowed=new Uint8Array(canvas.width*canvas.height);
   const {mergeRegisteredLineSegments}=await import('/src/lib/card-region-layout.js');
   for(const field of mergeRegisteredLineSegments(registration.fields))for(const line of field.lines){const left=Math.max(0,Math.floor(line.x*canvas.width)-2),right=Math.min(canvas.width,Math.ceil((line.x+line.width)*canvas.width)+2),top=Math.max(0,Math.floor(line.y*canvas.height)-2),bottom=Math.min(canvas.height,Math.ceil((line.y+line.height)*canvas.height)+2);for(let y=top;y<bottom;y++)allowed.fill(1,y*canvas.width+left,y*canvas.width+right);}
   let outside=0;for(let p=0;p<allowed.length;p++)if(!allowed[p]&&[0,1,2].some(k=>before[p*4+k]!==after[p*4+k]))outside++;return outside;
  },registration);
  assert.equal(changedOutside,0,c.name+' changed artwork outside registered text');
  await page.close();
 }}finally{await browser.close();await vite.close();}
});
