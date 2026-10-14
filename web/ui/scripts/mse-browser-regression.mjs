import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import fixtures from './mse-render-fixtures.mjs';
const args=process.argv.slice(2),fetchAssets=args.includes('--fetch-assets'),ordered=args.includes('--ordered-fields');
const only=args.find(v=>v.startsWith('--only='))?.slice(7);
const selected=only?fixtures.filter(f=>f.id.startsWith(only)):fixtures;
if(!selected.length)throw new Error('No matching fixtures');
const fontDpi=Number(args.find(v=>v.startsWith('--font-dpi='))?.slice(11)??96);
if(![72,96].includes(fontDpi))throw new Error('Font DPI must be 72 or 96');
const [corpus,output='reports/mse/browser-regression']=args.filter(v=>!v.startsWith('--'));
if(!corpus||args.some(v=>v.startsWith('--')&&!['--fetch-assets','--ordered-fields'].includes(v)&&!v.startsWith('--font-dpi=')&&!v.startsWith('--only=')))throw new Error('Usage: node mse-browser-regression.mjs CORPUS [OUTPUT_DIRECTORY] [--fetch-assets] [--ordered-fields] [--font-dpi=72|96] [--only=ID_PREFIX]');
const root=resolve(output),probe=fileURLToPath(new URL('./mse-browser-template-probe.mjs',import.meta.url));
await mkdir(root,{recursive:true});
const results=[];
for(const fixture of selected){
  const contextPath=resolve(root,fixture.id+'.context.json'),reportPath=resolve(root,fixture.id+'.json');
  await writeFile(contextPath,JSON.stringify(fixture.snapshot??{...fixture.context,...(ordered?{fieldUpdateMode:'ordered'}:{})},null,2)+'\n');
  const child=spawn(process.execPath,[probe,resolve(corpus),fixture.style,reportPath,`--${fixture.snapshot?'engine-card':'card'}=${contextPath}`,`--font-dpi=${fontDpi}`,`--face-index=${fixture.faceIndex??0}`,...(ordered?['--ordered-fields']:[]),...(fetchAssets?['--fetch-assets']:[])],{stdio:['ignore','ignore','pipe']});
  let errorText='';child.stderr.on('data',data=>{errorText+=data;});
  const code=await new Promise((done,reject)=>{child.on('error',reject);child.on('close',done);});
  let report;
  try{report=JSON.parse(await readFile(reportPath,'utf8'));}catch{report={};}
  const errors=[];
  if(code!==0||!report.success)errors.push(report.error?.message||errorText||`Probe exited ${code}`);
  if(report.success){
    if(JSON.stringify(report.dimensions)!==JSON.stringify(fixture.dimensions))errors.push('Unexpected canvas dimensions');
    for(const [field,expected] of Object.entries(fixture.text))if(report.measuredText?.[field]!==expected)errors.push(`Unexpected measured text for ${field}: ${JSON.stringify(report.measuredText?.[field])}`);
    if(fontDpi===72)for(const [field,expected] of Object.entries(fixture.layout||{})){
      if(expected.alignment!==undefined&&report.measuredLayouts?.[field]?.alignment!==expected.alignment)
        errors.push(`Native alignment mismatch for ${field}: ${report.measuredLayouts?.[field]?.alignment} versus ${expected.alignment}`);
      for(const [property,value] of Object.entries(expected.geometry||{})){
        const actual=report.measuredLayouts?.[field]?.geometry?.[property];
        if(!Number.isFinite(actual)||Math.abs(actual-value)>expected.tolerance)errors.push(`Native geometry mismatch for ${field}.${property}: ${actual} versus ${value}`);
      }
      const actual=report.measuredLayouts?.[field]?.lines;
      if(!actual||actual.length!==expected.lines.length)errors.push(`Unexpected native line count for ${field}`);
      else for(let row=0;row<actual.length;row++)for(let col=0;col<3;col++)
        if(!Number.isFinite(actual[row][col])||Math.abs(actual[row][col]-expected.lines[row][col])>expected.tolerance)
          errors.push(`Native layout mismatch for ${field}, line ${row}, metric ${col}: ${actual[row][col]} versus ${expected.lines[row][col]}`);
    }
    if(!report.imagePath)errors.push('Missing rendered artifact');
    for(const diagnostic of report.diagnostics||[])if(diagnostic.severity==='error')errors.push(diagnostic.message);
  }
  const result={id:fixture.id,style:fixture.style,success:errors.length===0,errors,reportPath,imagePath:report.imagePath};
  results.push(result);console.log(JSON.stringify(result));
}
const summary={selection:only??null,fontDpi,fieldUpdateMode:ordered?'ordered':'lazy',scope:'Populated layout-family smoke tests: drawing completion, dimensions, and selected measured text. Not native visual parity or full-pack coverage.',success:results.every(r=>r.success),results};
await writeFile(resolve(root,'summary.json'),JSON.stringify(summary,null,2)+'\n');
if(!summary.success)process.exitCode=1;
