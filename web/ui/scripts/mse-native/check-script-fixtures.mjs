import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,dirname,join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {formatCases} from '../../tests/fixtures/mse-format-cases.mjs';
import {plainTextCases} from '../../tests/fixtures/mse-plain-text-cases.mjs';
import {MseRuntime} from '../../src/lib/mse/script-runtime.mjs';
import {coreBuiltins} from '../../src/lib/mse/script-builtins.mjs';
import {code} from '../../src/lib/mse/script-values.mjs';

const [executableArg,setArg,outputArg]=process.argv.slice(2);
if(!outputArg)throw new Error('Usage: check-script-fixtures.mjs NATIVE_EXECUTABLE NATIVE_SET OUTPUT.json');
const executable=resolve(executableArg),set=resolve(setArg),output=resolve(outputArg);
const cases=[...formatCases.map(c=>({...c,family:'format'})),...plainTextCases.map(c=>({...c,family:'plain-text'}))];
const query=cases.map(c=>c.expression).join('\n')+'\n:quit\n';
const native=spawnSync(executable,['--cli',set,'--raw'],{input:query,encoding:'utf8',timeout:30000,maxBuffer:8*1024*1024});
const report={scope:'Native and JavaScript comparisons for recorded script fixtures only; not full scripting or rendering parity.',executable,set,
  executableSha256:createHash('sha256').update(await readFile(executable)).digest('hex'),
  setSha256:createHash('sha256').update(await readFile(join(set,'set'))).digest('hex'),
  query,exitCode:native.status,signal:native.signal,stdout:native.stdout,stderr:native.stderr,results:[]};
try{
  if(native.error)throw native.error;
  if(native.status!==0||native.stderr)throw new Error('Native process failed or emitted diagnostics');
  const lines=native.stdout.replaceAll('\r\n','\n').trimEnd().split('\n');let offset=0;
  const response=()=>{
    const status=lines[offset++],count=lines[offset++];
    if(!/^\d+$/.test(status)||!/^\d+$/.test(count)||offset+Number(count)>lines.length)throw new Error('Malformed native CLI response');
    const value=lines.slice(offset,offset+Number(count)).join('\n');offset+=Number(count);return {status:Number(status),value};
  };
  const runtime=new MseRuntime({builtins:coreBuiltins()});
  for(const fixture of cases){
    const nativeResult=response();let actual,error;
    try{actual=code(runtime.run(fixture.expression));}catch(e){error=e.message;}
    const expected=code(fixture.expected);
    report.results.push({family:fixture.family,expression:fixture.expression,expected,actual,error,native:nativeResult,
      success:!error&&nativeResult.status===0&&nativeResult.value===expected&&actual===expected});
  }
  const quit=response();if(quit.status!==0||quit.value||offset!==lines.length)throw new Error('Unexpected final CLI response');
  report.success=report.results.every(r=>r.success);
}catch(error){report.success=false;report.error=error.message;}
await mkdir(dirname(output),{recursive:true});await writeFile(output,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({success:report.success,checked:report.results.length,failures:report.results.filter(r=>!r.success),error:report.error,output}));
if(!report.success)process.exitCode=1;
