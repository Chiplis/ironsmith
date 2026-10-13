import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdir,readdir,rename,rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import upstream from '../src/lib/mse/upstream.mjs';
const args=process.argv.slice(2),fetchAssets=args.includes('--fetch-assets');
const [corpus,output='reports/mse/browser-audit']=args.filter(v=>!v.startsWith('--'));
if(!corpus||args.some(v=>v.startsWith('--')&&v!=='--fetch-assets'))throw new Error('Usage: node mse-browser-audit.mjs CORPUS [OUTPUT_DIRECTORY] [--fetch-assets]');
const tree=JSON.parse(await readFile(resolve(corpus,'upstream-tree.json'),'utf8'));
if(tree.sha!==upstream.revision||tree.truncated)throw new Error('Expected complete pinned corpus index');
const styles=tree.tree.filter(e=>e.type==='blob'&&/^data\/[^/]+\.mse-style\/style$/.test(e.path)).map(e=>e.path.split('/')[1]).sort();
const root=resolve(output),probe=fileURLToPath(new URL('./mse-browser-template-probe.mjs',import.meta.url));
const library=fileURLToPath(new URL('../src/lib/mse/',import.meta.url));
async function fingerprint(){
  const hash=createHash('sha256');hash.update(await readFile(probe));
  for(const file of (await readdir(library)).filter(n=>n.endsWith('.mjs')).sort()){hash.update(file);hash.update(await readFile(resolve(library,file)));}
  return hash.digest('hex');
}
const runtimeHash=await fingerprint(),startedAt=new Date().toISOString(),results=[];
await mkdir(root,{recursive:true});
async function save(complete=false){
  const failures={};for(const result of results)if(!result.success){const key=result.error?.code||'host';failures[key]=(failures[key]||0)+1;}
  const summary={upstream,runtimeHash,startedAt,updatedAt:new Date().toISOString(),complete,total:styles.length,checked:results.length,passed:results.filter(r=>r.success).length,failures,
    scope:'Every pinned pack stylesheet, default card, real Chromium drawing. A successful draw does not establish populated-card coverage or native visual parity.',results};
  const temp=resolve(root,'summary.json.tmp');await writeFile(temp,JSON.stringify(summary,null,2)+'\n');await rename(temp,resolve(root,'summary.json'));
}
await save();
for(const style of styles){
  if(await fingerprint()!==runtimeHash)throw new Error('Runtime changed during audit; restart for a consistent result');
  const reportPath=resolve(root,style+'.json');await rm(reportPath,{force:true});
  const child=spawn(process.execPath,[probe,resolve(corpus),style,reportPath,...(fetchAssets?['--fetch-assets']:[])],{stdio:['ignore','ignore','pipe']});
  let stderr='';child.stderr.on('data',data=>{stderr=(stderr+data).slice(-8192);});
  const code=await new Promise((done,reject)=>{child.on('error',reject);child.on('close',done);});
  let report;try{report=JSON.parse(await readFile(reportPath,'utf8'));}catch{report={};}
  const success=code===0&&report.success===true;
  const result={style,success,phase:report.phase,field:report.field,dimensions:report.dimensions,imagePath:report.imagePath,fontFailures:report.fontFailures,
    error:success?undefined:report.error||{code:'host',message:stderr||`Probe exited ${code}`},reportPath};
  results.push(result);await save();
  console.log(JSON.stringify({checked:results.length,total:styles.length,style,success,error:result.error}));
}
await save(true);
if(results.some(r=>!r.success))process.exitCode=1;
