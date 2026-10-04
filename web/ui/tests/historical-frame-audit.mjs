// Capture production previews and their sampled ink, using pinned offline scans.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {chromium} from 'playwright';
import {createServer} from 'vite';
const base=new URL('./fixtures/historical-frames/',import.meta.url);
let corpus=JSON.parse(await readFile(new URL('corpus.json',base),'utf8'));
const arnjlot={slug:'ice-arnjlot',local:'../frame-mask/arnjlot',printing:{id:'2307fb16-8b77-45b5-8a02-51a13214791d',set:'ice',collector_number:'61',lang:'en',name:"Arnjlot's Ascent",type_line:'Enchantment',mana_cost:'{1}{U}{U}',colors:['U'],frame:'1993',border_color:'black',layout:'normal',oracle_text:'Cumulative upkeep {U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\n{1}: Target creature gains flying until end of turn.',flavor_text:'"The dreams of a child fulfilled:\nthe wind on my brow,\nthe air ’neath my feet."\n—Arnjlot Olasson, Sky Mage',artist:'Drew Tucker'}};
arnjlot.printing.image_uris={normal:`https://cards.scryfall.io/normal/front/2/3/${arnjlot.printing.id}.jpg`,art_crop:`https://cards.scryfall.io/art_crop/front/2/3/${arnjlot.printing.id}.jpg`};
corpus=[arnjlot,...corpus];
if(process.env.FRAME_AUDIT_REPRESENTATIVES==='1') {
 const chosen=new Set(['ice-arnjlot','4ed-59','6ed-1','6ed-55','all-22a','all-64a','all-85','all-106','all-116a','atq-80a','arn-70','ath-65']);
 for(const set of new Set(corpus.map(c=>c.printing.set))) {
  const entries=corpus.filter(c=>c.printing.set===set&&!/-(en|de|fr|it|zhs)-/.test(c.slug));
  for(const entry of entries.slice(0,2))chosen.add(entry.slug);
 }
 for(const entry of corpus.filter(c=>/-(en|de|fr|it|zhs)-/.test(c.slug)))chosen.add(entry.slug);
 corpus=corpus.filter(c=>chosen.has(c.slug));
}
if(process.env.FRAME_AUDIT_FILTER)corpus=corpus.filter(c=>new RegExp(process.env.FRAME_AUDIT_FILTER).test(c.slug));
const out=process.env.FRAME_AUDIT_OUTPUT||'test-results/historical-frames/before';await mkdir(out,{recursive:true});
const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'silent'});await vite.listen();
const browser=await chromium.launch(),results=process.env.FRAME_AUDIT_RESUME==='1'?JSON.parse(await readFile(`${out}/results.json`,'utf8').catch(()=> '[]')):[];
corpus=corpus.filter(c=>!results.some(r=>r.slug===c.slug));
let writeQueue=Promise.resolve();
async function capture(entry){
 const p=entry.printing,page=await browser.newPage({viewport:{width:900,height:720},reducedMotion:'reduce'}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(p=>{localStorage.setItem('ironsmith.locale','en');window.__comparisonCards=[{...p,id:1,sourceImageUrl:p.image_uris.normal}];window.__comparisonShowOriginals=true;},p);
 if(process.env.FRAME_AUDIT_BASELINE==='1')await page.route('**/src/lib/card-frame-colors.js',async r=>{
  const response=await r.fetch();let body=await response.text();
  body=body.replace(/export function sectionInk\(region, \{preferredInk\} = \{\}\) \{[\s\S]*?const ink = analyzeSection/, 'export function sectionInk(region) {\n  const ink = analyzeSection');
  body=body.replace(', ...printingProfileInkStyle(cardPrintingProfile(printing))','').replace('Object.assign(style, printingProfileInkStyle(typography?.profile));','');
  await r.fulfill({response,body});
 });
 if(process.env.FRAME_AUDIT_BASELINE==='1')await page.route('**/src/lib/card-region-mask.js',async r=>{
  const response=await r.fetch();const body=(await response.text()).replace('const preferredInk=profileSectionInk(profile,field.kind);','const preferredInk=null;');
  await r.fulfill({response,body});
 });
 await page.route('https://**',async r=>{
  const u=r.request().url();
  if(u.startsWith('https://cards.scryfall.io/'))return r.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(new URL((entry.local||entry.slug)+(u.includes('/art_crop/')?(entry.local?'-art.jpg':'-art_crop.jpg'):'-normal.jpg'),base))});
  if(u.startsWith('https://svgs.scryfall.io/'))return r.fulfill({contentType:'image/svg+xml',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(new URL(p.set+'.svg',base))});
  if(u.startsWith('https://api.scryfall.com/'))return r.fulfill({json:u.includes('/sets/')?JSON.parse(await readFile(new URL(p.set+'-set.json',base),'utf8')):u.includes('/search')?{data:[p]}:p});
  return r.abort();
 });
 await page.route('**/cards/*.json',r=>r.fulfill({json:{scryfall:p}}));
 try {
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
  await page.waitForFunction(()=>document.querySelector('[data-printing-ready="true"]') && document.querySelector('[data-render-ready="true"]'),null,{timeout:30000});
  await page.evaluate(async()=>{await document.fonts.ready;await Promise.allSettled([...document.images].map(i=>i.decode()));});
  const metric=await page.evaluate(async p=>{
   const {prepareCardFrame}=await import('/src/lib/card-frame-preparation.js');
   const prepared=await prepareCardFrame(p.image_uris.art_crop,p.type_line),s=prepared.style||{};
   const stage=document.querySelector('.interactive-card-frame-stage');
   return {status:s['--source-frame-status'],reason:s['--source-frame-fallback-reason'],mode:stage?.dataset.frameMode,ink:Object.fromEntries(['title','type','rules','stats'].map(n=>[n,s[`--sampled-${n}-ink`]])),actual:Object.fromEntries(['title','type','rules','printed-stats'].map(n=>[n,[...document.querySelectorAll(`.interactive-card-frame__${n}`)].map(e=>getComputedStyle(e).color)])),profile:prepared.typography?.profile,geometry:s['--printed-layout']};
  },p);
  await page.screenshot({path:`${out}/${entry.slug}.png`,fullPage:true});
  await page.locator('[data-comparison-card="0"]').screenshot({path:`${out}/${entry.slug}-render.png`});
  results.push({slug:entry.slug,name:p.name,set:p.set,colors:p.colors,...metric,errors});
  console.log(entry.slug,JSON.stringify(metric.ink),metric.status);
 }catch(e){results.push({slug:entry.slug,error:e.message,errors});console.log(entry.slug,e.message);}
 const snapshot=JSON.stringify(results,null,2);writeQueue=writeQueue.then(()=>writeFile(`${out}/results.json`,snapshot));await writeQueue;await page.close();
}
const pending=[...corpus];
try {await Promise.all(Array.from({length:Number(process.env.FRAME_AUDIT_CONCURRENCY||3)},async()=>{while(pending.length)await capture(pending.shift());}));}
finally{await browser.close();await vite.close();}
