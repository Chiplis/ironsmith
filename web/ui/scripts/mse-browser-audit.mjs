import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdir,rename,rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {captureRuntimeSnapshot,runtimeFingerprint} from './mse-audit-snapshot.mjs';
import upstream from '../src/lib/mse/upstream.mjs';
const args=process.argv.slice(2),fetchAssets=args.includes('--fetch-assets'),ordered=args.includes('--ordered-fields');
const failuresFrom=args.find(a=>a.startsWith('--failures-from='))?.slice(16);
const [corpus,output='reports/mse/browser-audit']=args.filter(v=>!v.startsWith('--'));
if(!corpus||args.some(v=>v.startsWith('--')&&!['--fetch-assets','--ordered-fields'].includes(v)&&!v.startsWith('--failures-from=')))throw new Error('Usage: node mse-browser-audit.mjs CORPUS [OUTPUT_DIRECTORY] [--fetch-assets] [--ordered-fields] [--failures-from=COMPLETE_AUDIT.json]');
const tree=JSON.parse(await readFile(resolve(corpus,'upstream-tree.json'),'utf8'));
if(tree.sha!==upstream.revision||tree.truncated)throw new Error('Expected complete pinned corpus index');
let styles=tree.tree.filter(e=>e.type==='blob'&&/^data\/[^/]+\.mse-style\/style$/.test(e.path)).map(e=>e.path.split('/')[1]).sort();
let selection=null;
if(failuresFrom){
  const baseline=JSON.parse(await readFile(resolve(failuresFrom),'utf8'));
  if(!baseline.complete||baseline.upstream?.revision!==upstream.revision||baseline.total!==styles.length
    ||JSON.stringify(baseline.results.map(r=>r.style).sort())!==JSON.stringify(styles))
    throw new Error('Failure selection requires a complete full-pack audit at the pinned revision');
  selection={baseline:resolve(failuresFrom),runtimeHash:baseline.runtimeHash,total:styles.length};
  const failed=new Set(baseline.results.filter(r=>!r.success).map(r=>r.style));
  styles=styles.filter(s=>failed.has(s));
}
const root=resolve(output),probe=fileURLToPath(new URL('./mse-browser-template-probe.mjs',import.meta.url));
const library=fileURLToPath(new URL('../src/lib/mse/',import.meta.url));
const snapshot=await captureRuntimeSnapshot({probe,library,output:root,nodeModules:fileURLToPath(new URL('../node_modules',import.meta.url))});
const runtimeHash=snapshot.hash,startedAt=new Date().toISOString(),results=[];
const contextPath=resolve(snapshot.root,'audit-context.json');
await writeFile(contextPath,JSON.stringify({fieldUpdateMode:ordered?'ordered':'lazy'})+'\n');
async function save(complete=false){
  const failures={};for(const result of results)if(!result.success){const key=result.error?.code||'host';failures[key]=(failures[key]||0)+1;}
  const summary={upstream,selection,fieldUpdateMode:ordered?'ordered':'lazy',runtimeHash,runtimePath:snapshot.root,startedAt,updatedAt:new Date().toISOString(),complete,total:styles.length,checked:results.length,passed:results.filter(r=>r.success).length,failures,
    scope:(selection?'Previously failing styles selected from a completed full-pack audit':'Every pinned pack stylesheet')+', default card, real Chromium drawing. A successful draw does not establish populated-card coverage or native visual parity.',results};
  const temp=resolve(root,'summary.json.tmp');await writeFile(temp,JSON.stringify(summary,null,2)+'\n');await rename(temp,resolve(root,'summary.json'));
}
await save();
for(const style of styles){
  if(await runtimeFingerprint(snapshot.probe,snapshot.library)!==runtimeHash)throw new Error('Audit snapshot was modified');
  const reportPath=resolve(root,style+'.json');await rm(reportPath,{force:true});
  const child=spawn(process.execPath,[snapshot.probe,resolve(corpus),style,reportPath,`--card=${contextPath}`,...(fetchAssets?['--fetch-assets']:[])],{stdio:['ignore','ignore','pipe']});
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
