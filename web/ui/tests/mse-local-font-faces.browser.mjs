import {createServer} from 'node:http';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {dirname,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
// Requires installed Verdana faces; this explicit browser check reports an
// unavailable host font as a failure, not a successful rendering check.
const root=fileURLToPath(new URL('../src/lib/mse',import.meta.url));
const output=resolve(process.argv[2]||'reports/mse/local-font-faces.json');
const server=createServer(async(req,res)=>{try{if(req.url==='/'){res.end('<html><body></body></html>');return;}if(!/^\/[a-z-]+\.mjs$/.test(req.url))throw Error('path');res.setHeader('Content-Type','text/javascript');res.end(await readFile(root+req.url));}catch{res.writeHead(404);res.end();}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));let browser;
try{browser=await chromium.launch({headless:true});const page=await browser.newPage();await page.goto(`http://127.0.0.1:${server.address().port}`);
const result=await page.evaluate(async()=>{
 const {MseBrowserFonts}=await import('/browser-fonts.mjs');const fonts=new MseBrowserFonts({allowSystemFonts:true,pixelsPerPoint:1});const results=[];
 for(const [bold,italic,localName] of [[false,false,'Verdana'],[true,false,'Verdana Bold'],[false,true,'Verdana Italic'],[true,true,'Verdana Bold Italic']]){
  const row={bold,italic,localName};try{
   const provider=await fonts.prepare({name:'Verdana',size:20,bold,italic});const sample='AVW Wide 0123456789';row.providerWidth=provider.measure(sample,null,1).width;
   const ref=new FontFace('Reference'+results.length,`local(${JSON.stringify(localName)})`,{weight:bold?'700':'400',style:italic?'italic':'normal'});await ref.load();document.fonts.add(ref);
   const c=document.createElement('canvas').getContext('2d');c.font=`${italic?'italic':'normal'} ${bold?'bold':'normal'} 20px "${ref.family}"`;row.referenceWidth=Math.ceil(c.measureText(sample).width*4)/4;
   c.canvas.width=400;c.canvas.height=60;c.font=`${italic?'italic':'normal'} ${bold?'bold':'normal'} 20px "${ref.family}"`;
   c.textBaseline='alphabetic';c.fillText(sample,0,c.measureText(sample).fontBoundingBoxAscent);
   const actual=document.createElement('canvas');actual.width=400;actual.height=60;const ac=actual.getContext('2d');provider.draw(ac,sample,0,0,1);
   const expectedPixels=c.getImageData(0,0,400,60).data,actualPixels=ac.getImageData(0,0,400,60).data;
   row.identicalGlyphPixels=expectedPixels.every((v,i)=>v===actualPixels[i]);
   row.matches=row.providerWidth===row.referenceWidth&&row.identicalGlyphPixels;
  }catch(error){row.error=error.message;}results.push(row);
 }fonts.dispose();return results;
});await mkdir(dirname(output),{recursive:true});await writeFile(output,JSON.stringify({scope:'Renderer local-face selection versus exact installed Verdana faces in Chromium; not native MSE layout parity.',results:result},null,2)+'\n');console.log(result);assert.ok(result.every(r=>r.matches),'Local font-face metrics or glyph pixels differ from the installed reference face');
}finally{await browser?.close();await new Promise(r=>server.close(r));}
