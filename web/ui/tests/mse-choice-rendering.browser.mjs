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
    const {MseBrowserFonts}=await import('/browser-fonts.mjs');
    const results=[];
    for(const type of ['choice','slider'])for(const mode of ['text','image','both','both checklist','image hidden','checklist']){
      const docs={
        'test.mse-game/game':`card field:\n\tname: value\n\ttype: ${type}\n\tinitial: 100\n`,
        'test.mse-style/style':`game: test\ncard width: 64\ncard height: 24\ncard style:\n\tvalue:\n\t\tleft: 0\n\t\ttop: 0\n\t\twidth: 64\n\t\theight: 24\n\t\talignment: middle left\n\t\trender style: ${mode}\n\t\timage: {input + ".png"}\n`,
      };
      const fonts=new MseBrowserFonts({normalFont:{family:'sans-serif',pixelSize:12}});
      const assets=[];
      const service=new MseCardRenderer({store:new MsePackageStore({readText:async p=>docs[p.key]}),fonts,
        imageRenderer:new MseImageRenderer({loadAsset:async path=>{
          assets.push(path.key);const data=new Uint8ClampedArray(4*4*4);
          for(let i=0;i<data.length;i+=4)data.set([255,0,0,255],i);
          return {width:4,height:4,hasAlpha:false,data};
        }})});
      const result=await service.render({style:'test.mse-style'});
      const pixels=result.canvas.getContext('2d').getImageData(0,0,64,24).data;
      let red=0,black=0;
      for(let i=0;i<pixels.length;i+=4){
        if(pixels[i]===255&&pixels[i+1]===0&&pixels[i+2]===0)red++;
        if(pixels[i]<128&&pixels[i+1]<128&&pixels[i+2]<128)black++;
      }
      results.push({type,mode,red,black,assets,contentWidth:result.context.property('value','content_width')});
      service.dispose();fonts.dispose();
    }
    return results;
  });
  for(const r of results){
    const image=['image','both','both checklist','image hidden'].includes(r.mode);
    if(image)assert(r.assets.every(path=>path==='test.mse-style/100.png')&&r.assets.length>0,JSON.stringify(r));
    else assert.equal(r.assets.length,0,JSON.stringify(r));
    if(r.mode==='image hidden'){assert.equal(r.red,0);assert.equal(r.black,0);assert(r.contentWidth>0);}
    else if(r.mode.startsWith('both')){assert.equal(r.red,16,JSON.stringify(r));assert(r.black>0,JSON.stringify(r));assert.equal(r.contentWidth,4);}
    else if(r.mode==='image'){assert(r.red>16,JSON.stringify(r));assert.equal(r.black,0);}
    else {assert.equal(r.red,0);assert(r.black>0,JSON.stringify(r));}
  }
  console.log(JSON.stringify({cases:results.length,status:'passed',nativeVisualParityVerified:false}));
}finally{await browser?.close();await new Promise(done=>server.close(done));}
