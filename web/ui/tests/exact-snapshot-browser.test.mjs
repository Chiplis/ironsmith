import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { fileURLToPath } from 'node:url';

test('IndexedDB image survives a page refresh and a new production worker; corrupt restore falls back to clean replay', {timeout:120000}, async()=>{
 const vite=process.env.IRONSMITH_SNAPSHOT_TEST_URL ? null : await createServer({root:fileURLToPath(new URL('../',import.meta.url)),server:{host:'127.0.0.1',port:0},logLevel:'silent'});
 await vite?.listen();
 const origin=process.env.IRONSMITH_SNAPSHOT_TEST_URL || new URL(vite.resolvedUrls.local[0]).origin;
 const browser=await chromium.launch({headless:true});
 try {
  const page=await browser.newPage();
  await page.route(`${origin}/snapshot-test`,route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Snapshot recovery test</title>'}));
  await page.goto(`${origin}/snapshot-test`);
  async function client() {
   await page.evaluate(async()=>{
    const {createSnapshotDecoder}=await import('/src/lib/snapshot-channel.js');
    const decoder=createSnapshotDecoder(); const pending=new Map(); let id=0,generation=0;
    const worker=new Worker('/src/workers/wasmGameWorker.js',{type:'module'});
    const ready=new Promise((resolve,reject)=>{
     worker.onmessage=({data})=>{
      if(data.type==='ready') resolve(data);
      if(data.type==='error') reject(Error(data.error?.message));
      if(data.type==='result') {
       const entry=pending.get(data.id); if(!entry)return; pending.delete(data.id);
       if(!data.ok)entry.reject(Error(data.error.message));
       else entry.resolve(data.snapshot?decoder.decode(data.snapshot):data.result);
      }
     }; worker.onerror=event=>reject(Error(event.message));
    });
    worker.postMessage({type:'init',cardAssetsBaseUrl:location.origin+'/'});
    window.snapshotClient={worker,ready,call(method,...args){
     if(method==='restoreExactBuildSnapshot')generation++;
     return new Promise((resolve,reject)=>{const next=++id;pending.set(next,{resolve,reject});worker.postMessage({type:'call',id:next,method,args,runtimeGeneration:generation});});
    }};
    if(!(await ready).exactBuildSnapshots)throw Error('Missing snapshot capability');
   });
  }
  await client();
  const captured=await page.evaluate(async()=>{
   const store=await import('/src/lib/exact-snapshot-store.js');const c=window.snapshotClient;
   await c.call('resetEmpty',['Alice','Bob'],20);await c.call('addLifeDelta',0,5);
   const {publicCheckpointHash}=await import('/src/lib/multiplayer-audit.js');
   let wrongAnchorRejected=false;
   try { await c.call('captureExactBuildSnapshot',{publicStateHash:'wrong'}); }
   catch(error) { wrongAnchorRejected=/accepted public state/.test(error.message); }
   if(!wrongAnchorRejected)throw Error('Capture accepted a wrong public hash');
   const publicStateHash=await publicCheckpointHash(await c.call('exportPublicAuditCheckpoint'),crypto);
   const image=await c.call('captureExactBuildSnapshot',{matchId:'browser-test',seat:0,seq:1,publicStateHash});
   await store.writeExactSnapshot({matchId:'browser-test',seat:0,seq:1,image});
   c.worker.terminate(); return {bytes:image.memory.length,buildId:image.buildId};
  });
  assert.ok(captured.bytes>65536);assert.match(captured.buildId,/^[a-f0-9]{64}$/);
  await page.reload();await client();
  const result=await page.evaluate(async()=>{
   const store=await import('/src/lib/exact-snapshot-store.js');const c=window.snapshotClient;
   const saved=await store.readExactSnapshot('browser-test',0);
   let state=await c.call('restoreExactBuildSnapshot',saved.image);
   const restored=state.players[0].life;
   await c.call('addLifeDelta',0,2);state=await c.call('uiState');const replayed=state.players[0].life;
   saved.image.memory[0]^=1;await store.writeExactSnapshot(saved);
   const corrupt=await store.readExactSnapshot('browser-test',0);
   let rejected=false;
   try{await c.call('restoreExactBuildSnapshot',corrupt.image)}catch(error){rejected=/integrity/.test(error.message)}
   await c.call('resetEmpty',['Alice','Bob'],20);
   await c.call('addLifeDelta',0,5);await c.call('addLifeDelta',0,2);state=await c.call('uiState');
   await store.deleteExactSnapshot('browser-test',0);
   for(let index=0;index<5;index++)await store.writeExactSnapshot({matchId:`bounded-${index}`,seat:0,seq:index,image:{}});
   let retained=0;
   for(let index=0;index<5;index++)if(await store.readExactSnapshot(`bounded-${index}`,0))retained++;
   if(retained!==3)throw Error('Persistent images exceeded the retention bound');
   c.worker.terminate();return{restored,replayed,rejected,genesis:state.players[0].life};
  });
  assert.deepEqual(result,{restored:25,replayed:27,rejected:true,genesis:27});
 }finally{await browser.close();await vite?.close();}
});
