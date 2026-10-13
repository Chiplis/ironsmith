import {mkdir,realpath,symlink} from 'node:fs/promises';
import {resolve,join} from 'node:path';

const [build,source,corpus]=process.argv.slice(2);
if(!build||!source||!corpus)throw new Error('Usage: prepare-resources.mjs BUILD_DIRECTORY MSE_SOURCE CORPUS');
const root=resolve(build);
// wx macOS uses SharedSupport; other desktop hosts walk upward from the
// executable. Private fonts are discovered beside the executable on all hosts.
for(const [relative,target] of [
  ['data',join(corpus,'data')],['resource',join(source,'resource')],['Fonts',join(corpus,'Magic - Fonts')],
  ['SharedSupport/data',join(corpus,'data')],['SharedSupport/resource',join(source,'resource')],
]){
  const destination=join(root,relative),expected=await realpath(resolve(target));
  await mkdir(resolve(destination,'..'),{recursive:true});
  try{await symlink(expected,destination,'dir');}
  catch(error){if(error.code!=='EEXIST'||await realpath(destination)!==expected)throw error;}
}
console.log(JSON.stringify({build:root,resources:'ready'}));
