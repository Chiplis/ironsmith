import test from 'node:test';
import process from 'node:process';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
import {createServer} from 'vite';

// Real scan regression without checking copyrighted images into the repo.
// The fixture directory contains printing.json, normal.jpg and art_crop.jpg.
const historyCache=!process.env.CARD_FRAME_FUTURE_FIXTURE&&process.env.FRAME_HISTORY_CACHE;
const fixture=process.env.CARD_FRAME_FUTURE_FIXTURE||historyCache;
const fixtureFile=name=>join(fixture,historyCache
  ? name==='printing.json'?'feaba0f4-de2b-46a5-a728-04c8d699c523.json':'feaba0f4-de2b-46a5-a728-04c8d699c523-'+name
  : name);
test('Future Sight masks retain the original art, curved frame and off-title mana', {skip:!fixture,timeout:60000}, async()=>{
  const printing=JSON.parse(await readFile(fixtureFile('printing.json'),'utf8'));
  assert.equal(printing.frame,'future');
  const root=dirname(dirname(fileURLToPath(import.meta.url)));
  const vite=await createServer({root,server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:900,height:800}});
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.addInitScript(p=>{window.__comparisonCards=[{...p,id:1,sourceImageUrl:p.image_uris.normal}];},printing);
    await page.route('https://cards.scryfall.io/**',async route=>{
      assert.ok(route.request().url().includes(printing.id));
      const variant=route.request().url().includes('/art_crop/')?'art_crop':'normal';
      await route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(fixtureFile(variant+'.jpg'))});
    });
    await page.route('https://api.scryfall.com/**',route=>route.fulfill({json:route.request().url().includes(printing.id)?printing:{}}));
    await page.route('https://svgs.scryfall.io/**',route=>route.abort());
    await page.route('**/cards/*.json',route=>route.fulfill({json:{scryfall:printing}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`,{waitUntil:'domcontentloaded'});
    const stage=page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(()=>document.querySelector('[data-render-ready="true"]'));
    assert.equal(await stage.getAttribute('data-frame-mode'),'masked');
    assert.equal(await stage.locator('.original-card-fallback').count(),0);
    assert.equal(await stage.locator('.interactive-card-frame__mana').count(),0);
    const registration=await stage.evaluate(node=>({
      mana:JSON.parse(node.style.getPropertyValue('--printed-mana-symbols')),
      layout:JSON.parse(node.style.getPropertyValue('--printed-layout')),
      title:JSON.parse(node.style.getPropertyValue('--printed-title-text-bounds')),
      type:JSON.parse(node.style.getPropertyValue('--printed-type-text-bounds')),
      rulesSize:node.style.getPropertyValue('--printed-rules-font-size'),
      source:node.style.getPropertyValue('--source-frame-image'),
    }));
    assert.ok(registration.rulesSize,'symbol-led rules retain measured font sizing');
    assert.ok(registration.type.x>=registration.layout.type.x-6,
      'type text is measured inside the curved rail, not on its left rim');
    assert.ok(registration.mana.symbols.every(s=>s.x<registration.layout.art.x&&s.y>registration.layout.title.y));
    assert.equal(await stage.locator('.interactive-card-frame__source-mana img').count(),registration.mana.symbols.length);
    const checked=await page.evaluate(async({source,url,title,layout})=>{
      const load=async src=>{const img=new Image();img.crossOrigin='anonymous';img.src=src;await img.decode();return img;};
      const images=await Promise.all([load(source.slice(5,-2)),load(url)]);
      const scans=images.map(image=>{const canvas=document.createElement('canvas');canvas.width=488;canvas.height=680;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0,488,680);return ctx.getImageData(0,0,488,680).data;});
      for(const [x,y,w,h] of [[110,130,280,230],[20,200,8,160],[190,32,200,4],[110,610,180,40]])
        for(let py=y;py<y+h;py++)for(let px=x;px<x+w;px++)for(let c=0;c<4;c++)if(scans[0][(py*488+px)*4+c]!==scans[1][(py*488+px)*4+c])return {x:px,y:py,c,masked:scans[0][(py*488+px)*4+c],original:scans[1][(py*488+px)*4+c]};
      // These printings use dark text. Check original ink at the title and
      // left edge of the rules, where crop errors can leave whole initials.
      const regions=[title,{x:layout.rules.x+6,y:layout.rules.y+10,width:12,height:layout.rules.height-20}];
      const residuals=regions.map(b=>{
        let ink=0,left=0;
        for(let y=Math.ceil(b.y);y<Math.floor(b.y+b.height);y++)for(let x=Math.ceil(b.x);x<Math.floor(b.x+b.width);x++){
          const p=(y*488+x)*4;
          if(Math.max(...scans[1].subarray(p,p+3))<110){ink++;if(Math.max(...scans[0].subarray(p,p+3))<110)left++;}
        }
        return {ink,left};
      });
      return {unchanged:true,residuals};
    },{source:registration.source,url:printing.image_uris.normal,title:registration.title,layout:registration.layout});
    assert.equal(checked.unchanged,true,JSON.stringify(checked));
    for(const residual of checked.residuals){
      assert.ok(residual.ink>0,JSON.stringify(checked));
      assert.ok(residual.left/residual.ink<.03,JSON.stringify(checked));
    }
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
