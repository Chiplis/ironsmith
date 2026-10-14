import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {resolve,dirname} from 'node:path';
import {chromium} from 'playwright';
import {infoGame,infoStyle} from './fixtures/mse-info-field.mjs';
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
  const result=await page.evaluate(async({infoGame,infoStyle})=>{
    const {MsePackageStore}=await import('/packages.mjs');
    const {MseCardRenderer}=await import('/card-renderer.mjs');
    const {MseImageRenderer}=await import('/image-renderer.mjs');
    const {MseBrowserFonts}=await import('/browser-fonts.mjs');
    const docs={'mse-compat-info-test.mse-game/game':infoGame,'mse-compat-info-test-standard.mse-style/style':infoStyle};
    const fonts=new MseBrowserFonts({allowSystemFonts:true,pixelsPerPoint:1});
    const renderer=new MseCardRenderer({store:new MsePackageStore({readText:async p=>docs[p.key]}),fonts,imageRenderer:new MseImageRenderer({loadAsset:async()=>{throw new Error('Unexpected asset');}}),fieldUpdateMode:'ordered'});
    try{
      const canvas=document.createElement('canvas');
      await renderer.renderTo(canvas,{style:'mse-compat-info-test-standard.mse-style'});
      const ctx=canvas.getContext('2d');
      return {dimensions:[canvas.width,canvas.height],outside:[...ctx.getImageData(7,7,1,1).data],background:[...ctx.getImageData(8,8,1,1).data],png:canvas.toDataURL().split(',')[1]};
    }finally{renderer.dispose();fonts.dispose();}
  },{infoGame,infoStyle});
  assert.deepEqual(result.dimensions,[160,80]);assert.deepEqual(result.outside,[255,255,255,255]);assert.deepEqual(result.background,[220,230,240,255]);
  const output=resolve(process.argv[2]||'reports/mse/info-field-browser.json');
  await mkdir(dirname(output),{recursive:true});const imagePath=output.replace(/\.json$/,'.png');
  await writeFile(imagePath,Buffer.from(result.png,'base64'));delete result.png;
  await writeFile(output,JSON.stringify({scope:'Full browser card renderer with a scripted info field; native comparison recorded separately.',success:true,imagePath,...result},null,2)+'\n');
  console.log(JSON.stringify({status:'passed',output}));
}finally{await browser?.close();await new Promise(done=>server.close(done));}
