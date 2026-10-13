import {readFile,writeFile,mkdir,rm} from 'node:fs/promises';
import {resolve,join,dirname} from 'node:path';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';

const [executableArg,manifestArg,outputArg,sourceArg]=process.argv.slice(2);
if(!sourceArg)throw new Error('Usage: export-fixtures.mjs EXECUTABLE FIXTURE_MANIFEST OUTPUT_DIRECTORY MSE_SOURCE');
const executable=resolve(executableArg),manifestPath=resolve(manifestArg),output=resolve(outputArg);
const manifestBytes=await readFile(manifestPath),manifest=JSON.parse(manifestBytes);
const revision=spawnSync('git',['-C',resolve(sourceArg),'rev-parse','HEAD'],{encoding:'utf8'});
if(revision.status!==0||revision.stdout.trim()!==manifest.upstream.referenceImplementation.revision)throw new Error('Native source revision does not match fixture manifest');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const portability=JSON.parse(await readFile(join(sourceArg,'native-reference-portability.json'),'utf8'));
for(const patch of portability.patches){
  if(hash(await readFile(join(sourceArg,patch.file)))!==patch.patchedSha256)throw new Error(`Native source changed after patch record: ${patch.file}`);
}
await mkdir(output,{recursive:true});
const report={scope:'Native macOS reference exports. Font metrics differ from the Windows/Linux 96-DPI path; exports alone do not establish browser visual parity.',exportMethod:'write_image_file, zoom:100, use_user_settings:false',upstream:manifest.upstream,platform:process.platform,executable,executableSha256:hash(await readFile(executable)),manifestSha256:hash(manifestBytes),portability,results:[]};
for(const fixture of manifest.fixtures){
  if(!/^[a-z0-9-]+$/.test(fixture.id))throw new Error('Unsafe fixture ID');
  const imagePath=join(output,`${fixture.id}.png`),logPath=join(output,`${fixture.id}.log`);
  await rm(imagePath,{force:true}); // Never accept a previous run's export.
  const scriptPath=JSON.stringify(imagePath).replaceAll('{','\\{');
  const input=`write_image_file(input:set.cards.0,file:${scriptPath},zoom:100,use_user_settings:false)\n:quit\n`;
  const run=spawnSync(executable,['--cli',fixture.path,'--raw'],{input,cwd:dirname(executable),encoding:'utf8',timeout:60000,maxBuffer:8*1024*1024});
  const log=(run.stdout??'')+(run.stderr??'');
  await writeFile(logPath,log);
  const result={id:fixture.id,style:fixture.style,exitCode:run.status,signal:run.signal,imagePath,logPath,diagnostics:(run.stderr??'').trim(),inputSha256:hash(await readFile(join(fixture.path,'set')))};
  try{
    if(run.error)throw run.error;
    if(run.status!==0)throw new Error(`Native export exited ${run.status}`);
    const lines=(run.stdout??'').replaceAll('\r\n','\n').trimEnd().split('\n'),responses=[];
    for(let index=0;index<lines.length;){
      const status=lines[index++],count=lines[index++];
      if(!/^\d+$/.test(status)||!/^\d+$/.test(count)||index+Number(count)>lines.length)throw new Error('Invalid native CLI response');
      const payload=lines.slice(index,index+Number(count)).join('\n');index+=Number(count);
      responses.push({status:Number(status),payload});
    }
    result.responses=responses;
    const errors=responses.filter(response=>response.status!==0).map(response=>response.payload);
    if(errors.length)result.diagnostics=[result.diagnostics,...errors].filter(Boolean).join('\n');
    if(!responses.some(response=>response.status===0&&response.payload))throw new Error('Native CLI did not acknowledge the export');
    const bytes=await readFile(imagePath);
    if(bytes.subarray(0,8).toString('hex')!=='89504e470d0a1a0a')throw new Error('Export is not a PNG');
    result.dimensions=[bytes.readUInt32BE(16),bytes.readUInt32BE(20)];
    result.imageSha256=hash(bytes);
    if(String(result.dimensions)!==String(fixture.expectedDimensions))throw new Error(`Expected ${fixture.expectedDimensions}, received ${result.dimensions}`);
    result.success=true;
  }catch(error){result.success=false;result.error=error.message;}
  report.results.push(result);
  console.log(JSON.stringify({id:result.id,success:result.success,error:result.error,diagnostics:!!result.diagnostics}));
  await writeFile(join(output,'summary.json'),JSON.stringify(report,null,2)+'\n');
}
report.success=report.results.every(result=>result.success&&!result.diagnostics);
await writeFile(join(output,'summary.json'),JSON.stringify(report,null,2)+'\n');
if(!report.success)process.exitCode=1;
