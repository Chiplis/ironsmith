// Exercise changed text against every generated registration, preserving scans.
import {createHash} from 'node:crypto';
import {readFile,writeFile,mkdir,rename} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {chromium} from 'playwright';
import {createServer} from 'vite';
import registrations from '../src/lib/card-region-history.generated.js';
const root=fileURLToPath(new URL('../',import.meta.url)),cache=process.env.FRAME_HISTORY_CACHE;
if(!cache)throw Error('Set FRAME_HISTORY_CACHE');
const fidelity=process.env.FRAME_HISTORY_FIDELITY==='1';
const out=process.env.FRAME_HISTORY_OUTPUT||join(root,'test-results/frame-history-live');await mkdir(out,{recursive:true});
const corpus=JSON.parse(await readFile(new URL('./frame-history-corpus.json',import.meta.url)));
const results=process.env.FRAME_HISTORY_RESUME==='1'?JSON.parse(await readFile(join(out,'results.json')).catch(()=>'[]')):[];const vite=await createServer({root,server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();const context=await browser.newContext({viewport:{width:520,height:760}});
const sourceFiles=['src/lib/card-region-mask.js','src/lib/card-frame-font-mask.js','src/lib/card-region-layout.js','src/components/right-rail/RegisteredCardFrame.jsx','src/components/right-rail/registered-card-frame.css','src/components/right-rail/CardFrameRulesBox.jsx','tests/card-frame-live-regions.jsx','tests/card-frame-fidelity-text.js','src/lib/registered-printed-text.js'];
const sourceDigest=createHash('sha256').update((await Promise.all(sourceFiles.map(p=>readFile(join(root,p),'utf8')))).join('')).digest('hex');
const generationKey=r=>createHash('sha256').update(sourceDigest+JSON.stringify(r)+String(fidelity)).digest('hex');
const filter=process.env.FRAME_HISTORY_FILTER?new RegExp(process.env.FRAME_HISTORY_FILTER):null;
const pending=registrations.filter(r=>(!filter||filter.test(r.id+' '+r.layout+' '+r.set))&&!results.some(x=>x.slug===corpus.cases.find(c=>c.id===r.id&&c.face===r.face)?.slug&&x.generationKey===generationKey(r)&&!x.issues.includes('capture-failed')));let queue=Promise.resolve();
async function capture(registration){
 const c=corpus.cases.find(c=>c.id===registration.id&&c.face===registration.face),printing=JSON.parse(await readFile(join(cache,c.id+'.json'))),page=await context.newPage();let result={slug:c.slug,name:c.name,layout:c.layout,generationKey:generationKey(registration),issues:[]};
 try{
  await page.addInitScript(data=>{window.__regionFixture=data;localStorage.setItem('ironsmith.locale','en');},{registration,printing,fidelity,liveStats:fidelity?undefined:(printing.card_faces?.[registration.face??0]||printing).loyalty!=null?'12':'7/7',liveRules:registration.fields.filter(f=>f.kind==='rule').map(f=>fidelity?f.text:f.text+' Additional changed text.')});
  await page.route('https://cards.scryfall.io/**',async route=>route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(cache,c.slug+'-normal.jpg'))}));
  await page.route('https://api.scryfall.com/**',route=>route.fulfill({json:{}}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-live-regions.html`,{waitUntil:'domcontentloaded',timeout:90000});
  const expectedRules=registration.fields.filter(f=>f.kind==='rule'&&(!fidelity||!f.opaqueLettering)).length;
  const expectedHeaders=['name','type'].filter(kind=>registration.fields.filter(f=>f.kind===kind).length===1&&(!fidelity||!registration.fields.find(f=>f.kind===kind).opaqueHeader&&!registration.fields.find(f=>f.kind===kind).opaqueLettering));
  await page.waitForFunction(({count,headers})=>document.querySelectorAll('[data-field-kind="rule"][data-replaced="true"]').length===count&&headers.every(kind=>document.querySelector(`[data-field-kind="${kind}"][data-replaced="true"]`)),{count:expectedRules,headers:expectedHeaders},{timeout:60000});
  await page.waitForSelector('.registered-card-frame__scan[data-mask-ready="true"]');
  await page.evaluate(async()=>{
   await document.fonts.ready;await Promise.all([...document.images].map(image=>image.decode()));
   let previous='',stable=0;
   for(let frame=0;frame<180&&stable<12;frame++){
    await new Promise(requestAnimationFrame);
    const snapshot=JSON.stringify([...document.querySelectorAll('[data-field-kind]')].map(n=>[n.getAttribute('style'),n.querySelector('[data-text-overflow]')?.getAttribute('style'),n.querySelector('[data-text-overflow]')?.dataset.textOverflow]));
    stable=snapshot===previous?stable+1:0;previous=snapshot;
   }
  });
  result.fields=await page.locator('[data-field-kind]').evaluateAll(nodes=>nodes.map(n=>{const b=n.getBoundingClientRect(),surface=n.closest('.registered-card-frame__surface').getBoundingClientRect(),text=n.querySelector('.interactive-card-frame__rules'),column=n.closest('.registered-card-frame__column'),visible=column?.getBoundingClientRect()||b;return {fieldIndex:Number(n.dataset.registrationIndex),kind:n.dataset.fieldKind,fontSize:text?getComputedStyle(text).fontSize:null,fitScale:text?.style.getPropertyValue('--card-rules-fit-scale'),replaced:n.dataset.replaced==='true',text:n.dataset.liveText,rect:b.toJSON(),outside:visible.left<surface.left-2||visible.right>surface.right+2||visible.top<surface.top-2||visible.bottom>surface.bottom+2,horizontal:text?text.scrollWidth-text.clientWidth:0,vertical:text&&text.dataset.textOverflow==='true'&&getComputedStyle(text).overflowY!=='auto'?text.scrollHeight-text.clientHeight:0,scrolling:!!text&&getComputedStyle(text).overflowY==='auto'};}));
  for(const f of result.fields){if(f.outside)result.issues.push(f.kind+'-outside-card');if(f.horizontal>2)result.issues.push(f.kind+'-horizontal-overflow');if(f.vertical>2)result.issues.push(f.kind+'-clipped');}
  const mask=await page.locator('.registered-card-frame__scan').evaluate(async image=>{const copy=new Image();copy.crossOrigin='anonymous';copy.src=image.currentSrc||image.src;await copy.decode();const canvas=document.createElement('canvas');canvas.width=copy.naturalWidth;canvas.height=copy.naturalHeight;canvas.getContext('2d').drawImage(copy,0,0);return canvas.toDataURL('image/png');});
  await writeFile(join(out,c.slug+'-mask.png'),Buffer.from(mask.split(',')[1],'base64'));
  await page.locator('[data-live-frame]').screenshot({path:join(out,c.slug+'-live.png')});
 }catch(e){result.issues.push('capture-failed');result.error=e.message;}
 finally{await page.close();}
 const existing=results.findIndex(r=>r.slug===c.slug);if(existing>=0)results.splice(existing,1);results.push(result);const snapshot=JSON.stringify(results,null,2);queue=queue.then(async()=>{await writeFile(join(out,'results.json.tmp'),snapshot);await rename(join(out,'results.json.tmp'),join(out,'results.json'));});await queue;console.log(results.length,c.name,result.issues.join(','));
}
try{await Promise.all(Array.from({length:Number(process.env.FRAME_HISTORY_CONCURRENCY||6)},async()=>{while(pending.length)await capture(pending.shift());}));}finally{await browser.close();await vite.close();}
