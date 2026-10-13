// Compare decoded pixels with Pillow independently of the browser canvas path.
// Usage: node scripts/mse-check-png.mjs FILE_OR_DIRECTORY [...]
import {readFile,readdir,stat} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {decodePng} from '../src/lib/mse/png-decoder.mjs';
const inputs=process.argv.slice(2);
if(!inputs.length)throw new Error('Provide PNG files or directories to compare with Pillow');
async function* files(path){
  if((await stat(path)).isDirectory()){
    for(const entry of await readdir(path,{withFileTypes:true})){
      if(entry.isDirectory())yield* files(join(path,entry.name));
      else if(entry.isFile()&&/\.png$/i.test(entry.name))yield join(path,entry.name);
    }
  }else if(/\.png$/i.test(path))yield path;
}
let images=0,bytes=0;const failures=[];
for(const input of inputs)for await(const path of files(resolve(input))){
  try{
    const actual=await decodePng(await readFile(path));
    const expected=spawnSync('python3',['-c','from PIL import Image; import sys; sys.stdout.buffer.write(Image.open(sys.argv[1]).convert("RGBA").tobytes())',path],{maxBuffer:actual.data.length+65536});
    if(expected.error)throw expected.error;
    if(expected.status!==0)throw new Error(expected.stderr.toString());
    if(expected.stdout.length!==actual.data.length)throw new Error('RGBA byte count differs');
    const index=actual.data.findIndex((value,i)=>value!==expected.stdout[i]);
    if(index!==-1)throw new Error(`RGBA byte ${index}: expected ${expected.stdout[index]}, received ${actual.data[index]}`);
    images++;bytes+=actual.data.length;
  }catch(error){failures.push({path,message:error.message});}
}
console.log(JSON.stringify({images,bytes,failures},null,2));
if(failures.length||!images)process.exitCode=1;
