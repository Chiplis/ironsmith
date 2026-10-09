import test from 'node:test';
import assert from 'node:assert/strict';
import process from 'node:process';
import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
import {createServer} from 'vite';

const cache=process.env.FRAME_HISTORY_CACHE,id='35e88348-c82a-4f64-a82e-a661e6cef536';
test('Inventions erase black lettering on dark copper rails',{skip:!cache,timeout:90000},async()=>{
  const printing=JSON.parse(await readFile(join(cache,id+'.json'),'utf8'));
  const vite=await createServer({root:fileURLToPath(new URL('../',import.meta.url)),server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();const browser=await chromium.launch();
  try{
    const page=await browser.newPage();
    await page.route('https://cards.scryfall.io/**',async route=>route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(cache,id+'-'+(route.request().url().includes('/art_crop/')?'art_crop':'normal')+'.jpg'))}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    const result=await page.evaluate(async printing=>{
      const {sampleCardFramePixels}=await import('/src/lib/card-frame-colors.js');
      const {cardTypography}=await import('/src/lib/card-typography.js');
      const {manaTemplates}=await import('/src/lib/card-mana-match.js');
      const load=async url=>{const image=new Image();image.crossOrigin='anonymous';image.src=url;await image.decode();const c=document.createElement('canvas');c.width=488;c.height=Math.round(image.naturalHeight*488/image.naturalWidth);c.getContext('2d').drawImage(image,0,0,c.width,c.height);return c.getContext('2d').getImageData(0,0,c.width,c.height);};
      const typography=cardTypography(printing);
      await Promise.all(['title','type','rules'].map(k=>document.fonts.load(`${k==='rules'?400:typography.titleWeight} 40px ${typography[k]}`)));
      const original=await load(printing.image_uris.normal);
      const style=await sampleCardFramePixels({fullScan:original,artScan:await load(printing.image_uris.art_crop),printing,typography,icons:await manaTemplates(printing.mana_cost)});
      if(style['--source-frame-status']!=='masked')return {status:style['--source-frame-status']};
      const clean=await load(style['--source-frame-image'].slice(5,-2));
      const residuals=[{x:47,y:46,width:156,height:24},{x:45,y:394,width:180,height:21}].map(b=>{
        let ink=0,residual=0;
        for(let y=b.y;y<b.y+b.height;y++)for(let x=b.x;x<b.x+b.width;x++){
          const p=(y*488+x)*4;
          if(Math.max(...original.data.subarray(p,p+3))<60){ink++;if(Math.max(...clean.data.subarray(p,p+3))<60)residual++;}
        }
        return {ink,residual};
      });
      return {status:'masked',residuals};
    },printing);
    assert.equal(result.status,'masked');
    for(const r of result.residuals){assert.ok(r.ink>300,JSON.stringify(r));assert.ok(r.residual/r.ink<.03,JSON.stringify(r));}
  }finally{await browser.close();await vite.close();}
});
