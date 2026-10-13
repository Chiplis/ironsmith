import {readFile,writeFile,mkdir,readdir,mkdtemp,symlink,rm} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';

export async function runtimeFingerprint(probe,library){
  const hash=createHash('sha256');hash.update(await readFile(probe));
  for(const file of (await readdir(library)).filter(n=>n.endsWith('.mjs')).sort()){
    hash.update(file);hash.update(await readFile(resolve(library,file)));
  }
  return hash.digest('hex');
}
// Retain the exact audited source beside its report, so subsequent fixes can
// proceed without changing either the audit input or its reproducibility.
export async function captureRuntimeSnapshot({probe,library,output,nodeModules}){
  const expected=await runtimeFingerprint(probe,library);
  await mkdir(output,{recursive:true});
  const root=await mkdtemp(join(resolve(output),'.runtime-'));
  const snapshotProbe=join(root,'scripts/mse-browser-template-probe.mjs');
  const snapshotLibrary=join(root,'src/lib/mse');
  try{
    await mkdir(join(root,'scripts'),{recursive:true});await mkdir(snapshotLibrary,{recursive:true});
    await writeFile(snapshotProbe,await readFile(probe));
    for(const file of (await readdir(library)).filter(n=>n.endsWith('.mjs')).sort())
      await writeFile(join(snapshotLibrary,file),await readFile(join(library,file)));
    const actual=await runtimeFingerprint(snapshotProbe,snapshotLibrary);
    if(actual!==expected||await runtimeFingerprint(probe,library)!==expected)throw new Error('Runtime changed while taking audit snapshot; retry capture');
    await symlink(resolve(nodeModules),join(root,'node_modules'),'dir');
    return {root,probe:snapshotProbe,library:snapshotLibrary,hash:actual};
  }catch(error){await rm(root,{recursive:true,force:true});throw error;}
}
