import { createServer } from 'node:http';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { chromium } from 'playwright';
import upstream from '../src/lib/mse/upstream.mjs';
const [directory,output='reports/mse/fonts.json']=process.argv.slice(2);
if(!directory)throw new Error('Usage: node scripts/mse-audit-fonts.mjs CORPUS_DIRECTORY [REPORT.json]');
const root=resolve(directory),library=resolve(dirname(fileURLToPath(import.meta.url)),'../src/lib/mse');
const tree=JSON.parse(await readFile(resolve(root,'upstream-tree.json'),'utf8'));
if(tree.sha!==upstream.revision || tree.truncated)throw new Error('Font corpus must have a complete pinned tree');
const files=tree.tree.filter(e=>e.type==='blob'&&/\.(ttf|otf)$/i.test(e.path));
const server=createServer(async(req,res)=>{
  try{
    if(req.url==='/'){res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>MSE font audit</title>');return;}
    const match=/^\/font\/(\d+)$/.exec(req.url);
    if(match){
      const entry=files[Number(match[1])];if(!entry)throw new Error('Unknown font');
      const bytes=await readFile(resolve(root,entry.path));
      const hash=createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
      if(hash!==entry.sha)throw new Error('Font hash mismatch');
      res.end(bytes);return;
    }
    if(!/^\/[a-z-]+\.mjs$/.test(req.url)){res.writeHead(404);res.end();return;}
    res.setHeader('Content-Type','text/javascript');res.end(await readFile(resolve(library,basename(req.url))));
  }catch(error){res.writeHead(500);res.end(error.message);}
});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
let browser;
try{
  browser=await chromium.launch({headless:true});const page=await browser.newPage();
  const browserDiagnostics=[];
  page.on('console',message=>{if(message.type()==='warning'||message.type()==='error')browserDiagnostics.push(message.text());});
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const results=await page.evaluate(async files=>{
    const {MseBrowserFonts}=await import('/browser-fonts.mjs');
    const {readFontMetadata}=await import('/font-metadata.mjs');
    const results=[];
    for(let i=0;i<files.length;i++){
      const result={path:files[i].path,sha:files[i].sha},fonts=new MseBrowserFonts();
      try{
        const response=await fetch(`/font/${i}`);if(!response.ok)throw new Error(await response.text());
        const bytes=await response.arrayBuffer();result.metadata=readFontMetadata(bytes);
        result.repairs=(await fonts.registerFile(bytes)).repairs;
        const metrics=await fonts.prepare({name:result.metadata.family,size:15});
        result.extent=metrics.measure('Magic 0123456789',null,1);
        if(!Number.isFinite(result.extent.width)||result.extent.height<=0)throw new Error('Invalid font metrics');
        result.loaded=true;
      }catch(error){result.loaded=false;result.error=error.message;}
      finally{fonts.dispose();}
      results.push(result);
    }
    return results;
  },files);
  const report={upstream,generatedAt:new Date().toISOString(),scope:'Chromium font loading and basic measurement; does not establish native text or full-card rendering parity.',browserDiagnostics,summary:{fonts:results.length,loaded:results.filter(r=>r.loaded).length,failed:results.filter(r=>!r.loaded).length},results};
  await mkdir(dirname(resolve(output)),{recursive:true});await writeFile(output,JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify({...report.summary,report:resolve(output),failures:results.filter(r=>!r.loaded).map(r=>({path:r.path,error:r.error}))},null,2));
  if(report.summary.failed)process.exitCode=1;
}finally{await browser?.close();await new Promise(done=>server.close(done));}
