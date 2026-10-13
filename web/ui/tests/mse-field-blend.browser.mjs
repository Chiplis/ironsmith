import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
const library=new URL('../src/lib/mse/',import.meta.url);
const server=createServer(async(req,res)=>{
  if(req.url==='/'){res.setHeader('Content-Type','text/html');res.end('<!doctype html>');return;}
  if(!/^\/[a-z-]+\.mjs$/.test(req.url)){res.writeHead(404);res.end();return;}
  try{res.setHeader('Content-Type','text/javascript');res.end(await readFile(fileURLToPath(new URL(req.url.slice(1),library))));}
  catch{res.writeHead(500);res.end();}
});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
let browser;
try{
  browser=await chromium.launch({headless:true});const page=await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const results=await page.evaluate(async()=>{
    const {MsePackageStore}=await import('/packages.mjs');
    const {MseCardRenderer}=await import('/card-renderer.mjs');
    const {MseImageRenderer}=await import('/image-renderer.mjs');
    const results=[];
    for(const mode of ['field','script','nested','default','normal'])for(const angle of [0,90])for(const alpha of [0,128,255]){
      const image=mode==='field'?'overlay.png':mode==='nested'?'script: set_alpha(set_combine("overlay.png",combine:"multiply"),alpha:1)':`script: set_combine("overlay.png",combine:"${mode==='script'?'multiply':mode}")`;
      const combine=mode==='script'||mode==='nested'?'add':'multiply';
      const docs={
        'test.mse-game/game':'card field:\n\tname: overlay\n\ttype: choice\n\tchoice: selected\n',
        'test.mse-style/style':`game: test\ncard width: 4\ncard height: 4\ncard background: rgb(100,150,200)\ncard style:\n\toverlay:\n\t\tleft: 1\n\t\ttop: ${angle?3:1}\n\t\twidth: 2\n\t\theight: 2\n\t\tangle: ${angle}\n\t\trender style: image\n\t\timage: ${image}\n\t\tcombine: ${combine}\n`,
      };
      const service=new MseCardRenderer({store:new MsePackageStore({readText:async p=>docs[p.key]}),fonts:{},
        imageRenderer:new MseImageRenderer({loadAsset:async()=>({width:1,height:1,hasAlpha:true,data:new Uint8ClampedArray([128,64,255,alpha])})})});
      const target=new OffscreenCanvas(1,1);
      await service.renderTo(target,{style:'test.mse-style'});
      results.push({mode,angle,alpha,pixels:[...target.getContext('2d').getImageData(0,0,4,4).data]});service.dispose();
    }
    return results;
  });
  for(const {mode,angle,alpha,pixels} of results)for(let y=0;y<4;y++)for(let x=0;x<4;x++){
    const actual=pixels.slice((y*4+x)*4,(y*4+x+1)*4);
    const inside=x>=1&&x<3&&y>=1&&y<3;
    const blend=mode==='normal'?[128,64,255]:[50,37,200];
    const expected=inside?[100+(blend[0]-100)*alpha/255,150+(blend[1]-150)*alpha/255,200+(blend[2]-200)*alpha/255,255]:[100,150,200,255];
    for(let c=0;c<4;c++)assert.ok(Math.abs(actual[c]-Math.round(expected[c]))<=1,`mode=${mode} angle=${angle} alpha=${alpha} pixel=${x},${y} channel=${c}: ${actual[c]} != ${expected[c]}`);
  }
  console.log(JSON.stringify({cases:results.length,pixels:results.length*16,status:'passed',nativeVisualParityVerified:false}));
}finally{await browser?.close();await new Promise(done=>server.close(done));}
