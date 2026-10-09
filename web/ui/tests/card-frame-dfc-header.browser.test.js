import test from 'node:test';
import assert from 'node:assert/strict';
import process from 'node:process';
import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
import {createServer} from 'vite';
const cache=process.env.FRAME_HISTORY_CACHE;
const id='060f9675-4921-4cbb-bae2-54c85c679fd4';
test('header registration preserves face indicators and rejects misplaced showcase rails',{skip:!cache,timeout:90000},async()=>{
 const printing=JSON.parse(await readFile(join(cache,id+'.json'),'utf8'));
 const reverseControl=JSON.parse(await readFile(join(cache,'53b2e955-0106-4c28-8897-bcdafd96195f.json'),'utf8'));
 const showcase=JSON.parse(await readFile(join(cache,'9f104106-2922-404e-a959-5d6d071aad74.json'),'utf8'));
 const root=fileURLToPath(new URL('../',import.meta.url));
 const vite=await createServer({root,server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage();
  await page.route('https://cards.scryfall.io/**',async route=>{
   const url=route.request().url();const face=url.includes('/back/')?1:0;const printingId=url.includes(id)?id:url.includes(showcase.id)?showcase.id:reverseControl.id;
   return route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(cache,printingId+(printingId===showcase.id?'':'-'+face)+'-'+(url.includes('/art_crop/')?'art_crop':'normal')+'.jpg'))});
  });
  await page.route('**/fixture-symbol.svg',async route=>route.fulfill({contentType:'image/svg+xml',body:await readFile(join(cache,'lcc.svg'))}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
  const results=await page.evaluate(async ({printing,reverseControl,showcase})=>{
   const {sampleCardFramePixels}=await import('/src/lib/card-frame-colors.js');
   const {cardTypography}=await import('/src/lib/card-typography.js');
   const {manaTemplates}=await import('/src/lib/card-mana-match.js');
   const load=async (url,width=488)=>{const i=new Image();i.crossOrigin='anonymous';i.src=url;await i.decode();const c=document.createElement('canvas');c.width=width;c.height=Math.round(i.naturalHeight*width/i.naturalWidth);c.getContext('2d').drawImage(i,0,0,c.width,c.height);return c.getContext('2d').getImageData(0,0,c.width,c.height);};
   const results=[];
   for(const [owner,face] of [...printing.card_faces.map(f=>[printing,f]),[reverseControl,reverseControl.card_faces[1]]]){
    const p={...owner,...face},typography=cardTypography(p);
    await Promise.all(['title','type','rules'].map(k=>document.fonts.load(`${k==='rules'?400:typography.titleWeight} 40px ${typography[k]}`)));
    const original=await load(face.image_uris.normal);
    const style=await sampleCardFramePixels({fullScan:original,artScan:await load(face.image_uris.art_crop),printing:p,typography,symbolScan:owner.id===reverseControl.id?await load('/fixture-symbol.svg',48):null,icons:await manaTemplates(p.mana_cost)});
    const title=JSON.parse(style['--printed-title-text-bounds']||'null');
    const clean=style['--source-frame-image']?await load(style['--source-frame-image'].slice(5,-2)):null;
    let indicatorChanged=0;
    if(clean)for(let y=40;y<58;y++)for(let x=36;x<59;x++)for(let c=0;c<4;c++)if(clean.data[(y*488+x)*4+c]!==original.data[(y*488+x)*4+c])indicatorChanged++;
    let colorIndicatorChanged=0;
    if(clean&&owner.id===reverseControl.id)for(let y=398;y<407;y++)for(let x=40;x<52;x++)for(let c=0;c<4;c++)if(clean.data[(y*488+x)*4+c]!==original.data[(y*488+x)*4+c])colorIndicatorChanged++;
    results.push({colorIndicatorChanged,status:style['--source-frame-status'],title,indicatorChanged,type:JSON.parse(style['--printed-type-text-bounds']||'null')});
   }
   const rejected=await sampleCardFramePixels({fullScan:await load(showcase.image_uris.normal),artScan:await load(showcase.image_uris.art_crop),printing:showcase,typography:cardTypography(showcase),icons:[]});
   results.push({guard:rejected['--source-frame-status'],reason:rejected['--source-frame-fallback-reason']});
   return results;
  },{printing,reverseControl,showcase});
  for(const [index,r] of results.entries()){
   if(index===3){assert.equal(r.guard,'unmasked');assert.equal(r.reason,'type-art-registration');continue;}
   assert.equal(r.status,'masked',JSON.stringify(r));
   // Independent positions from the pinned source scans / Vision OCR.
   assert.ok(Math.abs(r.title.x-[80,76,42][index])<=8,JSON.stringify(r));
   if(index<2)assert.equal(r.indicatorChanged,0,'face indicator pixels are preserved');
   if(index===2){
    assert.ok(r.type.x>=60&&r.type.x+r.type.width>430,JSON.stringify(r));
    assert.equal(r.colorIndicatorChanged,0,'type color indicator pixels are preserved');
   }
  }
 }finally{await browser.close();await vite.close();}
});
