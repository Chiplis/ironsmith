import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile,mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {resolve,dirname} from 'node:path';
import {chromium} from 'playwright';
import {legacySymbolCases} from './fixtures/mse-legacy-symbols.mjs';
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
  const results=await page.evaluate(async cases=>{
    const {createBrowserSymbolRenderer}=await import('/browser-symbols.mjs');
    const results=[];
    for(const fixture of cases)for(const local of [false,true]){
      const render=createBrowserSymbolRenderer({store:{readText:async p=>fixture[p.key]},loadLocalSymbol:async name=>fixture[name]});
      const request={local,variation:{type:'solid',borderRadius:0.05,fill_color:[0,0,0,255],border_color:[255,255,255,255]}};
      const old=await render({...request,filename:'legacy',path:{key:'legacy'}},{width:40,height:30});
      const modern=await render({...request,filename:'modern',path:{key:'modern'}},{width:40,height:30});
      let differentBytes=0,visiblePixels=0;
      for(let i=0;i<old.data.length;i++){if(old.data[i]!==modern.data[i])differentBytes++;if(i%4===3&&old.data[i])visiblePixels++;}
      results.push({id:fixture.id,local,dimensions:[old.width,old.height],modernDimensions:[modern.width,modern.height],differentBytes,visiblePixels});
    }
    return results;
  },legacySymbolCases);
  for(const result of results){assert.deepEqual(result.dimensions,result.modernDimensions);assert.equal(result.differentBytes,0);assert.ok(result.visiblePixels>0);}
  const output=resolve(process.argv[2]||'reports/mse/legacy-symbols-browser.json');
  await mkdir(dirname(output),{recursive:true});await writeFile(output,JSON.stringify({scope:'Legacy versus modern symbol raster equivalence in Chromium; not cross-renderer pixel parity.',success:true,results},null,2)+'\n');
  console.log(JSON.stringify({cases:results.length,status:'passed',output}));
}finally{await browser?.close();await new Promise(done=>server.close(done));}
