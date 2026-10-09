import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
import {createServer} from 'vite';

// External fixture: Scryfall MB2 #97 printing.json, normal.jpg, art_crop.jpg.
const fixture=process.env.CARD_FRAME_BAUBLE_FIXTURE;
test('Mishra’s Bauble clears the complete bottom attribution and preserves the frame',
  {skip:!fixture,timeout:60000},async()=>{
    const printing=JSON.parse(await readFile(join(fixture,'printing.json'),'utf8'));
    assert.equal(printing.id,'607e2546-1139-4c1e-9b3b-eacb27bff510');
    const vite=await createServer({root:dirname(dirname(fileURLToPath(import.meta.url))),server:{host:'127.0.0.1',port:0},logLevel:'silent'});
    await vite.listen();
    const browser=await chromium.launch();
    try {
      const page=await browser.newPage();
      await page.addInitScript(p=>{window.__comparisonCards=[{...p,id:1,sourceImageUrl:p.image_uris.normal}];},printing);
      await page.route('https://cards.scryfall.io/**',async route=>route.fulfill({
        contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},
        body:await readFile(join(fixture,route.request().url().includes('/art_crop/')?'art_crop.jpg':'normal.jpg')),
      }));
      await page.route('https://api.scryfall.com/**',route=>route.fulfill({json:route.request().url().includes(printing.id)?printing:{}}));
      await page.route('https://svgs.scryfall.io/**',route=>route.abort());
      await page.route('**/cards/*.json',route=>route.fulfill({json:{scryfall:printing}}));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
      await page.waitForSelector('[data-render-ready="true"]');
      const stage=page.locator('.interactive-card-frame-stage');
      assert.equal(await stage.getAttribute('data-frame-mode'),'masked');
      const result=await stage.evaluate(async(node,url)=>{
        const source=node.style.getPropertyValue('--source-frame-image').slice(5,-2);
        const scans=await Promise.all([url,source].map(async src=>{
          const image=new Image();image.crossOrigin='anonymous';image.src=src;await image.decode();
          const canvas=document.createElement('canvas');canvas.width=488;canvas.height=680;
          const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0,488,680);
          return ctx.getImageData(0,0,488,680).data;
        }));
        let ink=0,residual=0,changedProtected=0;
        for(let y=601;y<621;y++)for(let x=40;x<300;x++){
          const p=(y*488+x)*4;
          if(Math.max(...scans[0].subarray(p,p+3))<110){
            ink++;
            if(Math.max(...scans[1].subarray(p,p+3))<110)residual++;
          }
        }
        for(const [x,y,w,h] of [[50,90,380,270],[8,100,10,450],[30,634,430,40]])
          for(let py=y;py<y+h;py++)for(let px=x;px<x+w;px++)for(let c=0;c<4;c++){
            const p=(py*488+px)*4+c;
            if(scans[0][p]!==scans[1][p])changedProtected++;
          }
        return {ink,residual,changedProtected};
      },printing.image_uris.normal);
      assert.ok(result.ink>100,JSON.stringify(result));
      assert.equal(result.residual,0,JSON.stringify(result));
      assert.equal(result.changedProtected,0,JSON.stringify(result));
    }finally{await browser.close();await vite.close();}
  });
