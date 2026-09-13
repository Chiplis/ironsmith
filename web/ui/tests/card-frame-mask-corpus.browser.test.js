import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdir} from 'node:fs/promises';
import {chromium} from 'playwright';
import {createServer} from 'vite';
import corpus from './fixtures/frame-mask/corpus.js';

test('real printing masks remove joined words or reject the frame, across scan qualities', {timeout:90000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:1000,height:1000}});
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
  const results=await page.evaluate(async corpus=>{
   const {fontGuidedPanel}=await import('/src/lib/card-frame-font-mask.js');
   const results=[];document.body.innerHTML='<main style="background:#303030;padding:20px;color:white"></main>';
   for(const entry of corpus)for(const quality of [null,.45]) {
    const img=new Image();img.src='/tests/fixtures/frame-mask/'+entry.file;await img.decode();
    const c=document.createElement('canvas');c.width=img.width;c.height=img.height;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);
    if(quality){const compressed=new Image();compressed.src=c.toDataURL('image/jpeg',quality);await compressed.decode();ctx.drawImage(compressed,0,0);}
    await document.fonts.load(`${entry.options.weight} 40px ${entry.options.family}`);
    const scan=ctx.getImageData(0,0,c.width,c.height),clean=fontGuidedPanel(scan,entry.options);
    // Independently count remaining dark cores, including components the
    // recognizer never accepted. Do not merely assert its own quality flag.
    let originalInk=0,left=0,changedOutside=0;
    for(let p=0;p<scan.data.length;p+=4){if(p/4%c.width>=3&&p/4%c.width<c.width-3&&Math.floor(p/4/c.width)>=3&&Math.floor(p/4/c.width)<c.height-3&&scan.data[p]+scan.data[p+1]+scan.data[p+2]<270){originalInk++;if(clean.data[p]+clean.data[p+1]+clean.data[p+2]<270)left++;}if(!clean.mask[p/4]&&[0,1,2,3].some(k=>scan.data[p+k]!==clean.data[p+k]))changedOutside++;}
    const row=document.createElement('div');row.textContent=entry.file+' '+(quality?'JPEG 45':'original')+' '+(clean.quality.safe?'masked':'fallback');row.append(c);
    const output=document.createElement('canvas');output.width=c.width;output.height=c.height;output.getContext('2d').putImageData(new ImageData(clean.data,c.width,c.height),0,0);row.append(output);document.querySelector('main').append(row);
    results.push({file:entry.file,quality,safe:clean.quality.safe,expectedSafe:entry.expectedSafe,originalInk,left,changedOutside});
   }
   for(const [label,paper,ink,font] of [
    ['classic-white','#746443','#fff7e8','Georgia'],
    ['modern-gold','#d6be77','#17120a','"Beleren"'],
    ['modern-silver','#dededb','#181818','"Beleren"'],
   ])for(const scale of [1,2]) {
    const c=document.createElement('canvas');c.width=320*scale;c.height=44*scale;
    const ctx=c.getContext('2d');ctx.fillStyle=paper;ctx.fillRect(0,0,c.width,c.height);
    await document.fonts.load(`700 ${24*scale}px ${font}`);ctx.font=`700 ${24*scale}px ${font}`;
    ctx.fillStyle=ink;ctx.fillText('Human Cleric',8*scale,30*scale);
    ctx.fillRect(0,c.height-2,c.width,1);
    const scan=ctx.getImageData(0,0,c.width,c.height);
    const clean=fontGuidedPanel(scan,{family:font,weight:700,text:'Human Cleric',section:'type'});
    let borderChanged=0;for(let x=0;x<c.width;x++)if(clean.mask[(c.height-2)*c.width+x])borderChanged++;
    results.push({file:label+'-'+scale,quality:null,safe:clean.quality.safe,expectedSafe:true,originalInk:1,left:clean.quality.residual,changedOutside:borderChanged});
    const row=document.createElement('div');row.textContent=label+' '+scale+'x';row.append(c);
    const output=document.createElement('canvas');output.width=c.width;output.height=c.height;output.getContext('2d').putImageData(new ImageData(clean.data,c.width,c.height),0,0);row.append(output);document.querySelector('main').append(row);
   }
   return results;
  },corpus);
  await mkdir('test-results/frame-mask',{recursive:true});
  await page.setViewportSize({width:1000,height:2600});
  await page.screenshot({path:'test-results/frame-mask/corpus.png',fullPage:true});
  for(const r of results){assert.equal(r.changedOutside,0,JSON.stringify(r));if(r.quality===null)assert.equal(r.safe,r.expectedSafe,JSON.stringify(r));if(r.safe)assert.ok(r.left/r.originalInk<.03,JSON.stringify(r));}
 }finally{await browser.close();await vite.close();}
});
