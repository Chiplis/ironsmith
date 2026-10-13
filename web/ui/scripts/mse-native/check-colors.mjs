import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,dirname} from 'node:path';
import {createHash} from 'node:crypto';
import {namedColors} from '../../src/lib/mse/named-colors.mjs';
import {color} from '../../src/lib/mse/color-values.mjs';
import {text} from '../../src/lib/mse/script-values.mjs';
const [binary,reportArg]=process.argv.slice(2);
if(!binary||!reportArg)throw new Error('Usage: check-colors.mjs MSE_EXECUTABLE REPORT.json');
const values=[...Object.keys(namedColors).flatMap(name=>[name,name.toUpperCase()]),'transparent','rgb(-1,+2,259)','rgba(-1,258,+3,-128)','rgb(1,2,3,4)','#f008','#12345678'];
const input=values.map(value=>`to_string(to_color(${JSON.stringify(value)}))`).join('\n')+'\n:quit\n';
const run=spawnSync(resolve(binary),['--cli','--raw'],{input,encoding:'utf8',timeout:60000,maxBuffer:4*1024*1024});
if(run.error)throw run.error;
assert.equal(run.status,0,run.stderr);
assert.equal(run.stderr,'');
const lines=run.stdout.replaceAll('\r\n','\n').trimEnd().split('\n'),responses=[];
for(let i=0;i<lines.length;){
  const status=lines[i++],size=lines[i++];
  assert.match(status,/^\d+$/);assert.match(size,/^\d+$/);assert(i+Number(size)<=lines.length);
  responses.push({status:Number(status),payload:lines.slice(i,i+Number(size)).join('\n')});i+=Number(size);
}
assert.equal(responses.length,values.length+1);
assert.deepEqual(responses.at(-1),{status:0,payload:''});
const results=values.map((value,index)=>{
  const response=responses[index],expected=text(color(value));
  const actual=response.status===0?JSON.parse(response.payload):response.payload;
  return {value,expected,actual,success:response.status===0&&actual===expected};
});
const report={scope:'Native CLI differential check of named and fully specified numeric color parsing; incomplete scanf inputs have undefined native channels and are excluded.',executable:resolve(binary),executableSha256:createHash('sha256').update(await readFile(resolve(binary))).digest('hex'),cases:results.length,success:results.every(result=>result.success),results};
await mkdir(dirname(resolve(reportArg)),{recursive:true});await writeFile(reportArg,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({success:report.success,cases:report.cases,failures:results.filter(result=>!result.success)}));
if(!report.success)process.exitCode=1;
