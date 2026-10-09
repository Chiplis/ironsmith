// Offline production renderer audit. Build fixtures with build-frame-history-corpus.py.
// FRAME_HISTORY_CACHE is required. FRAME_HISTORY_FILTER limits families/names.
// FRAME_HISTORY_RESUME=1 keeps completed captures; outputs never declare a
// masked/custom frame visually correct merely because preparation succeeded.
import {readFile,writeFile,mkdir,access} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {createServer} from 'vite';
import {chromium} from 'playwright';
const root=fileURLToPath(new URL('../',import.meta.url));
const cache=process.env.FRAME_HISTORY_CACHE;
if(!cache)throw Error('Set FRAME_HISTORY_CACHE');
const out=process.env.FRAME_HISTORY_OUTPUT||join(root,'test-results/frame-history');
await mkdir(out,{recursive:true});
const corpus=JSON.parse(await readFile(new URL('./frame-history-corpus.json',import.meta.url),'utf8'));
const results=process.env.FRAME_HISTORY_RESUME==='1'?JSON.parse(await readFile(join(out,'results.json'),'utf8').catch(()=>'[]')):[];
const filter=process.env.FRAME_HISTORY_FILTER?new RegExp(process.env.FRAME_HISTORY_FILTER):null;
const registeredOnly=process.env.FRAME_HISTORY_REGISTRATIONS_ONLY==='1'?(await import('../src/lib/card-region-history.generated.js')).default:null;
const cases=corpus.cases.filter(c=>(!registeredOnly||registeredOnly.some(r=>r.id===c.id&&r.face===c.face))&&(!filter||filter.test(c.id+' '+c.name+' '+c.layout+' '+c.families.join(' ')))&&!results.some(r=>r.slug===c.slug));
const expected=results.length+cases.length;
const metadata=new Map();for(const c of cases)if(!metadata.has(c.id))metadata.set(c.id,JSON.parse(await readFile(join(cache,c.id+'.json'),'utf8')));
const vite=await createServer({root,server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'silent'});await vite.listen();
const browser=await chromium.launch();const context=await browser.newContext({viewport:{width:950,height:730},deviceScaleFactor:1,reducedMotion:'reduce'});let writeQueue=Promise.resolve();
async function capture(c){
 const printing=metadata.get(c.id),face=c.face==null?printing:printing.card_faces[c.face];
 const page=await context.newPage(),errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 const card={...face,id:1,sourceImageUrl:c.source};
 await page.addInitScript(card=>{window.__comparisonCards=[card];window.__comparisonShowOriginals=true;localStorage.setItem('ironsmith.locale','en');},card);
 await page.route('https://**',async route=>{
   const u=route.request().url();
   if(u.startsWith('https://cards.scryfall.io/')){
     const matched=corpus.cases.find(k=>u.includes(k.id)&&u.includes(k.source.includes('/back/')?'/back/':'/front/'));
     if(!matched)return route.abort();
     const path=join(cache,matched.slug+'-'+(u.includes('/art_crop/')?'art_crop':'normal')+'.jpg');
     try{return await route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(path)});}catch{return route.abort();}
   }
   if(u.startsWith('https://svgs.scryfall.io/')){
     try{return await route.fulfill({contentType:'image/svg+xml',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(cache,printing.set+'.svg'))});}catch{return route.abort();}
   }
   if(u.startsWith('https://api.scryfall.com/')){
     if(u.includes('/sets/'))return route.fulfill({json:JSON.parse(await readFile(join(cache,printing.set+'-set.json'),'utf8'))});
     return route.fulfill({json:u.includes('/search')?{data:[printing]}:printing});
   }
   return route.abort();
 });
 await page.route('**/cards/*.json',route=>route.fulfill({json:{scryfall:printing}}));
 let result={...c,errors};
 try{
   await access(join(cache,c.slug+'-normal.jpg'));await access(join(cache,c.slug+'-art_crop.jpg'));
   await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
   await page.waitForFunction(()=>{const n=document.querySelector('.interactive-card-frame-stage');return n?.dataset.renderReady==='true'&&n?.dataset.printingReady==='true';},null,{timeout:60000});
   await page.evaluate(async()=>{await document.fonts.ready;await Promise.allSettled([...document.images].map(i=>i.decode()));await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));});
   const metric=await page.locator('.interactive-card-frame-stage').evaluate(async n=>{
     const get=k=>n.style.getPropertyValue(k),parse=k=>JSON.parse(get(k)||'null');
     const frame=n.querySelector('.interactive-card-frame')||n.querySelector('.registered-card-frame'),bounds=frame?.getBoundingClientRect();
     const issues=[];
     const fields=['title','type','stats-text'].flatMap(kind=>[...n.querySelectorAll('.interactive-card-frame__'+kind)].map(el=>{
       const r=el.getBoundingClientRect(),css=getComputedStyle(el);const range=document.createRange();range.selectNodeContents(el);const text=range.getBoundingClientRect();
       const clipped=el.scrollWidth>el.clientWidth+2;
       if(clipped)issues.push(kind+'-horizontal-overflow');
       if(bounds&&(text.left<bounds.left-2||text.right>bounds.right+2||text.top<bounds.top-2||text.bottom>bounds.bottom+2))issues.push(kind+'-outside-card');
       return {kind,text:el.textContent,font:css.fontSize,rect:r.toJSON(),ink:text.toJSON(),clipped};
     }));
     const source=get('--source-frame-image');
     let mask=null;
     if(source){const image=new Image();image.src=source.slice(5,-2);await image.decode();const canvas=document.createElement('canvas');canvas.width=image.naturalWidth;canvas.height=image.naturalHeight;canvas.getContext('2d').drawImage(image,0,0);mask=canvas.toDataURL('image/png');}
     if(['custom','placeholder','original'].includes(n.dataset.frameMode))issues.push('source-frame-not-live');
     const rules=n.querySelector('.interactive-card-frame__rules');
     return {mode:n.dataset.frameMode,reason:n.dataset.frameFallbackReason||get('--source-frame-fallback-reason')||null,boxes:parse('--printed-layout'),titleBounds:parse('--printed-title-text-bounds'),typeBounds:parse('--printed-type-text-bounds'),symbolBounds:parse('--printed-set-symbol-bounds'),fields,issues,scrolling:rules?rules.scrollHeight>rules.clientHeight+2:false,mask};
   });
   const {mask,...metricWithoutImage}=metric;Object.assign(result,metricWithoutImage);
   if(mask)await writeFile(join(out,c.slug+'-mask.png'),Buffer.from(mask.split(',')[1],'base64'));
   await page.screenshot({path:join(out,c.slug+'.png'),fullPage:true});
   await page.locator('[data-comparison-card]').screenshot({path:join(out,c.slug+'-render.png')});
 }catch(e){result.error=e.message;result.issues=['capture-failed'];}
 finally{await page.close();}
 results.push(result);const snapshot=JSON.stringify(results,null,2);writeQueue=writeQueue.then(()=>writeFile(join(out,'results.json'),snapshot));await writeQueue;
 console.log(`${results.length}/${expected} ${c.name}: ${result.mode||'error'} ${(result.issues||[]).join(',')}`);
}
const pending=[...cases];
try{await Promise.all(Array.from({length:Number(process.env.FRAME_HISTORY_CONCURRENCY||4)},async()=>{while(pending.length)await capture(pending.shift());}));}
finally{await browser.close();await vite.close();}
console.log('Captured',results.length,'faces in',out);
