import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import fixtures from './mse-render-fixtures.mjs';
const args=process.argv.slice(2),fetchAssets=args.includes('--fetch-assets');
const [corpus,output='reports/mse/browser-regression']=args.filter(v=>!v.startsWith('--'));
if(!corpus||args.some(v=>v.startsWith('--')&&v!=='--fetch-assets'))throw new Error('Usage: node mse-browser-regression.mjs CORPUS [OUTPUT_DIRECTORY] [--fetch-assets]');
const root=resolve(output),probe=fileURLToPath(new URL('./mse-browser-template-probe.mjs',import.meta.url));
await mkdir(root,{recursive:true});
const results=[];
for(const fixture of fixtures){
  const contextPath=resolve(root,fixture.id+'.context.json'),reportPath=resolve(root,fixture.id+'.json');
  await writeFile(contextPath,JSON.stringify(fixture.context,null,2)+'\n');
  const child=spawn(process.execPath,[probe,resolve(corpus),fixture.style,reportPath,`--card=${contextPath}`,...(fetchAssets?['--fetch-assets']:[])],{stdio:['ignore','ignore','pipe']});
  let errorText='';child.stderr.on('data',data=>{errorText+=data;});
  const code=await new Promise((done,reject)=>{child.on('error',reject);child.on('close',done);});
  let report;
  try{report=JSON.parse(await readFile(reportPath,'utf8'));}catch{report={};}
  const errors=[];
  if(code!==0||!report.success)errors.push(report.error?.message||errorText||`Probe exited ${code}`);
  if(report.success){
    if(JSON.stringify(report.dimensions)!==JSON.stringify(fixture.dimensions))errors.push('Unexpected canvas dimensions');
    for(const [field,expected] of Object.entries(fixture.text))if(report.measuredText?.[field]!==expected)errors.push(`Unexpected measured text for ${field}: ${JSON.stringify(report.measuredText?.[field])}`);
    if(!report.imagePath)errors.push('Missing rendered artifact');
  }
  const result={id:fixture.id,style:fixture.style,success:errors.length===0,errors,reportPath,imagePath:report.imagePath};
  results.push(result);console.log(JSON.stringify(result));
}
const summary={scope:'Populated layout-family smoke tests: drawing completion, dimensions, and selected measured text. Not native visual parity or full-pack coverage.',success:results.every(r=>r.success),results};
await writeFile(resolve(root,'summary.json'),JSON.stringify(summary,null,2)+'\n');
if(!summary.success)process.exitCode=1;
