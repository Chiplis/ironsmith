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
    const {prepareFontDrawing}=await import('/font-painter.mjs');
    const {MseColor}=await import('/script-values.mjs');
    const fonts={prepare:async()=>({measure:()=>({width:1,height:1}),draw:(ctx,text,x,y,scale)=>ctx.fillRect(x,y,scale,scale)})};
    const results=[];
    for(const radius of [0,1]){
      const font={size:15,color:new MseColor(255,0,0),stroke_radius:radius,stroke_blur:0,stroke_color:new MseColor(0,0,255),shadow_color:new MseColor(0,255,0),shadow_displacement_x:3,shadow_displacement_y:0,shadow_blur:0};
      const provider=await prepareFontDrawing(font,fonts);
      const canvas=new OffscreenCanvas(9,9),ctx=canvas.getContext('2d');
      provider.draw(ctx,'X',2,2,1);provider.draw(ctx,'X',2,6,1);
      results.push({radius,pixels:[...ctx.getImageData(0,0,9,9).data]});
    }
    return results;
  });
  for(const {radius,pixels} of results)for(const y of [2,6]){
    const at=(x,y)=>pixels.slice((y*9+x)*4,(y*9+x+1)*4);
    assert.deepEqual(at(2,y),[255,0,0,255]);
    assert.deepEqual(at(5,y),radius?[0,0,0,0]:[0,255,0,255]);
    assert.deepEqual(at(3,y),radius?[0,0,255,255]:[0,0,0,0]);
  }
  const decorations=await page.evaluate(async()=>{
    const {MseBrowserFonts}=await import('/browser-fonts.mjs');
    const {prepareFontDrawing}=await import('/font-painter.mjs');
    const {MseColor}=await import('/script-values.mjs');
    const fonts=new MseBrowserFonts({canvas:new OffscreenCanvas(1,1),normalFont:{family:'serif',pixelSize:20}});
    const font={name:'',size:15,strikethrough:true,color:new MseColor(255,0,0)};
    const results=[];
    for(const shadow of [false,true]){
      const styled=shadow?{...font,shadow_color:new MseColor(0,255,0),shadow_displacement_x:0,shadow_displacement_y:4}:font;
      const provider=await prepareFontDrawing(styled,fonts),canvas=new OffscreenCanvas(80,50),ctx=canvas.getContext('2d');
      ctx.fillStyle='red';provider.draw(ctx,'    ',10,10,1);
      const pixels=ctx.getImageData(0,0,80,50).data;
      let red=0,green=0;for(let i=0;i<pixels.length;i+=4){if(pixels[i]&&pixels[i+3])red++;if(pixels[i+1]&&pixels[i+3])green++;}
      results.push({shadow,red,green});
    }
    fonts.dispose();return results;
  });
  for(const result of decorations){assert.ok(result.red>0);assert.equal(result.green>0,result.shadow);}
  console.log(JSON.stringify({cases:4,repeatedDraws:4,decorations,status:'passed',nativeGlyphParityVerified:false}));
}finally{await browser?.close();await new Promise(done=>server.close(done));}
