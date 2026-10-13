import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm,readlink} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {captureRuntimeSnapshot,runtimeFingerprint} from '../scripts/mse-audit-snapshot.mjs';

test('audit snapshots retain exact input while working renderer files change',async()=>{
  const root=await mkdtemp(join(tmpdir(),'mse-audit-snapshot-test-'));
  try{
    const probe=join(root,'probe.mjs'),library=join(root,'library'),output=join(root,'reports');
    await mkdir(library);await writeFile(probe,'original probe');await writeFile(join(library,'a.mjs'),'original runtime');
    const snapshot=await captureRuntimeSnapshot({probe,library,output,nodeModules:join(root,'node_modules')});
    assert.equal(snapshot.hash,await runtimeFingerprint(probe,library));
    await writeFile(probe,'new probe');await writeFile(join(library,'a.mjs'),'new runtime');await writeFile(join(library,'b.mjs'),'new module');
    assert.equal(snapshot.hash,await runtimeFingerprint(snapshot.probe,snapshot.library));
    assert.notEqual(snapshot.hash,await runtimeFingerprint(probe,library));
    assert.equal(await readFile(join(snapshot.library,'a.mjs'),'utf8'),'original runtime');
    assert.equal(await readlink(join(snapshot.root,'node_modules')),join(root,'node_modules'));
  }finally{await rm(root,{recursive:true,force:true});}
});
