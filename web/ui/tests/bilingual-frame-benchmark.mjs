// Offline, pinned full-card benchmark. Run from web/ui.
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {chromium} from 'playwright';
import {createServer} from 'vite';
import corpus from './fixtures/bilingual-frames/corpus.js';
import inkRegions from './fixtures/bilingual-frames/ink-regions.js';
const base=new URL('./fixtures/bilingual-frames/',import.meta.url);
const out=process.env.FRAME_BENCHMARK_OUTPUT||'test-results/bilingual-frames';await mkdir(out,{recursive:true});
const selected=corpus.filter(c=>!process.env.FRAME_BENCHMARK_FILTER||new RegExp(process.env.FRAME_BENCHMARK_FILTER).test(c.slug));
if(!selected.length)throw Error('No benchmark cards matched the filter');
const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'silent'});await vite.listen();
const browser=await chromium.launch({args:process.env.CARD_FRAME_GPU==='1'?['--enable-unsafe-webgpu',...(process.platform==='darwin'?['--use-angle=metal']:[])]:[]});const results=[];
try{for(const entry of selected){
 const p=entry.printing,page=await browser.newPage({viewport:{width:900,height:720},reducedMotion:'reduce'});
 if(process.env.CARD_FRAME_WORKER==='0')await page.addInitScript(()=>{window.Worker=undefined;});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(p=>{localStorage.setItem('ironsmith.locale',p.lang);window.__comparisonCards=[{...p,id:1,sourceImageUrl:p.image_uris.normal}];window.__comparisonShowOriginals=true;},p);
 await page.route('https://**',async route=>{
  const url=route.request().url();
  if(url.startsWith('https://cards.scryfall.io/')){const c=corpus.find(c=>url.includes(c.printing.id));if(!c)return route.abort();return route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(new URL(c.slug+(url.includes('/art_crop/')?'-art_crop.jpg':'-normal.jpg'),base))});}
  if(url.startsWith('https://svgs.scryfall.io/'))return route.fulfill({contentType:'image/svg+xml',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(new URL(p.set+'.svg',base))});
  if(url.startsWith('https://api.scryfall.com/')){
   const u=new URL(url);let c=corpus.find(c=>u.pathname.endsWith('/'+c.printing.id)||u.pathname.endsWith(`/${c.printing.set}/${c.printing.collector_number}/${c.printing.lang}`));
   if(u.pathname.startsWith('/sets/'))return route.fulfill({json:{icon_svg_uri:entry.setSymbol}});
   if(u.pathname==='/cards/search')return route.fulfill({json:{data:corpus.filter(c=>c.printing.name===p.name && c.printing.lang===(u.searchParams.get('q')?.includes('lang:es')?'es':'en')).map(c=>c.printing)}});
   if(u.pathname==='/cards/named')c=corpus.find(c=>c.printing.name===p.name&&c.printing.lang==='en');
   return route.fulfill({status:c?200:404,json:c?.printing||{}});
  }
  return route.abort();
 });
 if(process.env.FRAME_MASK_DIAGNOSTICS==='1')await page.route('**/src/lib/card-frame-font-mask.js',async route=>{
  const response=await route.fetch();let body=await response.text();
  body=body.replace('return {...result,quality,matches:',`if(!quality.safe)(globalThis.__maskFailures??=[]).push({section,quality,failed:textComponents.filter(c=>!residualTextQuality(scan,result,[c],paperAt,{outlined}).safe).map(({points,...c})=>c),components:components.map(({points,...c})=>c)});return {...result,quality,matches:`);
  await route.fulfill({response,body});
 });
 await page.route('**/cards/*.json',route=>route.fulfill({json:{}}));
 await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
 await page.waitForFunction(()=>document.querySelector('[data-render-ready="true"]'),{},{timeout:60000});
 await page.evaluate(async()=>{await document.fonts.ready;await Promise.allSettled([...document.images].map(i=>i.decode()));await new Promise((resolve,reject)=>{
   let last='',stable=0,frames=0;const check=()=>{const next=JSON.stringify([...document.querySelectorAll('.interactive-card-frame__title,.interactive-card-frame__type,.interactive-card-frame__rules')].map(e=>[e.getBoundingClientRect().toJSON(),getComputedStyle(e).fontSize]));stable=next===last?stable+1:0;last=next;if(stable>=4)return resolve();if(++frames>120)return reject(Error('Frame layout did not settle'));requestAnimationFrame(check);};requestAnimationFrame(check);
  });});
 const metrics=await page.evaluate(async ({p,regions:sourceRegions})=>{
  const {prepareCardFrame}=await import('/src/lib/card-frame-preparation.js');
  const prepared=await prepareCardFrame(p.image_uris.art_crop,p.type_line);
  let retryReason=null;
  if(prepared.printing?.lang&&prepared.printing.lang!==p.lang){
   const {sampleCardFrameColors}=await import('/src/lib/card-frame-colors.js');
   const native=await sampleCardFrameColors(p.image_uris.normal,{typography:prepared.typography,printing:p});
   retryReason=native?.['--source-frame-fallback-reason']||null;
  }
  const stage=document.querySelector('.interactive-card-frame-stage');
  const style=prepared.style||{};
  const read=async src=>{const i=new Image();i.crossOrigin='anonymous';i.src=src;await i.decode();const c=document.createElement('canvas');c.width=488;c.height=680;c.getContext('2d').drawImage(i,0,0,488,680);return c.getContext('2d').getImageData(0,0,488,680);};
  const regions={},decorationPixels=[];let artChanged=null,statsGeometry=null,decorationChanged=null;
  if(style['--source-frame-image']){
   const original=await read(prepared.printing?.image_uris?.normal || prepared.originalImageUrl),masked=await read(style['--source-frame-image'].slice(5,-2));
   const {detectPrintedStats,detectStatsPanelBounds}=await import('/src/lib/card-frame-colors.js');
   const stats=detectPrintedStats(original);statsGeometry={stats,panel:detectStatsPanelBounds(original,stats)};
   // Independent fixed art interior of the actual source printing, including retries.
   //, safely away from text and frame borders.
   artChanged=0;for(let y=130;y<340;y++)for(let x=85;x<405;x++){const at=(y*488+x)*4;if([0,1,2].some(k=>Math.abs(original.data[at+k]-masked.data[at+k])>8))artChanged++;}
   // Reviewed conventional-frame security stamp and P/T bevel bands. These
   // are independent of the detected panel bounds that drive the renderer.
   decorationChanged=0;
   const protectedBands=[];
   if(['m19','m20','m21'].includes(p.set))protectedBands.push([391,388,451,415]);
   if(['dmr','m19','m20','m21'].includes(p.set)) {
    if(['rare','mythic'].includes(p.rarity))protectedBands.push([216,618,273,646]);
    if(p.power!=null)protectedBands.push([375,610,392,640],[385,607,450,610]);
   }
   for(const [x0,y0,x1,y1]of protectedBands)for(let y=y0;y<y1;y++)for(let x=x0;x<x1;x++){
    const at=(y*488+x)*4;if([0,1,2].some(k=>Math.abs(original.data[at+k]-masked.data[at+k])>8)){decorationChanged++;decorationPixels.push([x,y]);}
   }
   // This is a screening metric, not a correctness oracle. Fixed interior
   // reviewed source bands do not call the renderer's glyph classifier or quality checker.
   for(const [name,[x0,y0,x1,y1]]of Object.entries(sourceRegions || {})){
    let ink=0,left=0;for(let y=y0;y<y1;y++)for(let x=x0;x<x1;x++){const at=(y*488+x)*4;if(original.data[at]+original.data[at+1]+original.data[at+2]<240){ink++;if(masked.data[at]+masked.data[at+1]+masked.data[at+2]<240)left++;}}
    regions[name]={ink,left,fraction:ink?left/ink:0};
   }
  }
  const fallback=stage?.dataset.frameMode==='original';
  const rulesOverflow=[...document.querySelectorAll('.interactive-card-frame__rules')].some(e=>{
    const box=e.getBoundingClientRect(),paddingBottom=parseFloat(getComputedStyle(e).paddingBottom)||0,stats=e.closest('.interactive-card-frame__rules-section')?.querySelector('.interactive-card-frame__printed-stats')?.getBoundingClientRect(),walker=document.createTreeWalker(e,NodeFilter.SHOW_TEXT);
    while(walker.nextNode())if(walker.currentNode.textContent.trim()){
      const range=document.createRange();range.selectNodeContents(walker.currentNode);
      if([...range.getClientRects()].some(r=>r.width>0&&(r.top<box.top-2||r.bottom>box.bottom-(stats?4:paddingBottom)+2||stats&&r.right>stats.left-3&&r.left<stats.right+3&&r.bottom>stats.top-2)))return true;
    }return false;
  });
  const reminders=[...document.querySelectorAll('.rules-reminder-text')].map(e=>{const cs=getComputedStyle(e), rows=[];const walker=document.createTreeWalker(e,NodeFilter.SHOW_TEXT);while(walker.nextNode()){const node=walker.currentNode;for(let i=0;i<node.length;i++){const range=document.createRange();range.setStart(node,i);range.setEnd(node,i+1);const r=range.getBoundingClientRect();let row=rows.find(row=>Math.abs(row.y-r.y)<1);if(!row){row={y:r.y,height:r.height,text:'',left:r.left,right:r.right};rows.push(row);}row.text+=node.textContent[i];row.right=Math.max(row.right,r.right);}}return {font:cs.font,fontFamily:cs.fontFamily,fontSize:cs.fontSize,letterSpacing:cs.letterSpacing,wordSpacing:cs.wordSpacing,rows};});
  return {reminders,typography:Object.fromEntries(Object.entries(style).filter(([k])=>k.includes('rules')||k.includes('flavor')||k.includes('reminder')||k.includes('backend'))),rules:[...document.querySelectorAll('.interactive-card-frame__rules,.interactive-card-frame__rule-line')].map(e=>({text:e.textContent,font:getComputedStyle(e).fontSize,family:getComputedStyle(e).fontFamily,fontStyle:getComputedStyle(e).fontStyle,letterSpacing:getComputedStyle(e).letterSpacing,width:e.clientWidth,lineHeight:getComputedStyle(e).lineHeight,style:e.style.cssText})),retryReason,alignment:{frame:stage?.querySelector('.interactive-card-frame')?.getBoundingClientRect().toJSON(),type:stage?.querySelector('.interactive-card-frame__type')?.getBoundingClientRect().toJSON(),typeTransform:stage?.querySelector('.interactive-card-frame__type')?.style.cssText,scan:stage?.style.getPropertyValue('--printed-scan-width'),baseline:stage?.style.getPropertyValue('--printed-type-baseline')},decorationChanged,decorationPixels,statsGeometry,sourceImageStatus:p.image_status,rulesOverflow,geometry:Object.fromEntries(Object.entries(style).filter(([k])=>['--printed-layout','--printed-title-text-bounds','--printed-type-text-bounds','--printed-mana-symbols'].includes(k))),diagnostics:globalThis.__maskFailures||[],mode:stage?.dataset.frameMode,reason:stage?.getAttribute('data-frame-fallback-reason')||style['--source-frame-fallback-reason']||null,printingLang:prepared.printing?.lang,regions,artChanged,fallback,overflow:[...document.querySelectorAll('.interactive-card-frame__title,.interactive-card-frame__type')].some(e=>e.scrollWidth>e.clientWidth+2)};
 },{p,regions:inkRegions[entry.slug]});
 await page.screenshot({path:`${out}/${entry.slug}.png`,fullPage:true});
 results.push({annotated:!!inkRegions[entry.slug],slug:entry.slug,name:p.name,lang:p.lang,...metrics,errors});console.log(entry.slug,JSON.stringify(metrics));await page.close();
}}finally{await browser.close();await vite.close();}
const summary=Object.fromEntries(['en','es'].map(lang=>{const rows=results.filter(r=>r.lang===lang);return [lang,{total:rows.length,originalFallbacks:rows.filter(r=>r.fallback).length,englishRetries:rows.filter(r=>r.printingLang!==lang).length,overflow:rows.filter(r=>r.overflow||r.rulesOverflow).length,artChanged:rows.filter(r=>r.artChanged>0).length,decorationChanged:rows.filter(r=>r.decorationChanged>0).length,errors:rows.filter(r=>r.errors.length).length}];}));
await writeFile(`${out}/results.json`,JSON.stringify({summary,results},null,2));
const esc=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
await writeFile(`${out}/index.html`,`<!doctype html><meta charset="utf-8"><title>English / Spanish frame benchmark</title><style>body{background:#121416;color:white;font:16px system-ui;max-width:1000px;margin:auto}img{width:100%}pre{white-space:pre-wrap}</style><h1>English / Spanish full-card benchmark</h1><p>Original left, rendered right. Residual counts are screening signals; inspect each image. Fallbacks on these conventional layouts are candidates for improvement.</p><pre>${esc(JSON.stringify(summary,null,2))}</pre>${results.map(r=>`<h2>${esc(r.slug)} — ${esc(r.mode)} ${esc(r.reason||'')}</h2><img src="${r.slug}.png">`).join('')}`);
console.log(JSON.stringify(summary));
if(results.some(r=>r.errors.length||r.artChanged>0||r.decorationChanged>0))process.exitCode=1;
if(process.env.FRAME_BENCHMARK_ASSERT==='1'&&results.some(r=>!r.annotated||r.fallback||r.overflow||r.rulesOverflow||!(r.alignment?.type?.height>0)||Object.values(r.regions).some(region=>region.left>3&&region.fraction>.03)))process.exitCode=1;
