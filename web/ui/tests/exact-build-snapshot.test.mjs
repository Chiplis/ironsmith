import test from 'node:test';
import assert from 'node:assert/strict';
import { sealExactSnapshot, validateExactSnapshot, snapshotDataBytes, exactSnapshotMatches } from '../src/lib/exact-build-snapshot.js';
import { recoverVerifiedRuntime } from '../src/lib/local-runtime-recovery.js';
const fixture = () => ({ version:1, buildId:'build', pointer:8, memory:new Uint8Array(65536), globals:[42], references:[[undefined,{value:1n}]], recovery:{secret:new Map([['key','private']])} });

test('integrity covers memory, globals, reference data and matching JS recovery state',async () => {
 const image=await sealExactSnapshot(fixture());
 await validateExactSnapshot(structuredClone(image),'build');
 for (const mutate of [s=>s.memory[0]++,s=>s.globals[0]++,s=>s.references[0][1].value++,s=>s.recovery.secret.set('key','other')]) {
  const copy=structuredClone(image); mutate(copy);
  await assert.rejects(validateExactSnapshot(copy,'build'),/integrity/);
 }
 await assert.rejects(validateExactSnapshot(image,'other'),/incompatible/);
});
test('reference hashing preserves bigint, undefined, aliases and cycles and rejects host resources', () => {
 const shared={n:2n}, value={a:shared,b:shared}; value.self=value;
 assert.deepEqual(snapshotDataBytes(value),snapshotDataBytes(structuredClone(value)));
 assert.notDeepEqual(snapshotDataBytes({a:undefined}),snapshotDataBytes({a:null}));
 assert.throws(()=>snapshotDataBytes(()=>{}),/unsupported/);
});
test('persistent images cannot cross seats, matches, or signed transcript anchors', () => {
 const point={matchId:'m',seat:0,seq:1,prefixHash:'prefix',auditStateHash:'state',publicStateHash:'public'};
 const context={matchId:'m',seat:0,actions:[{prefixHash:'prefix',audit:{nextStateHash:'state',publicCheckpointHash:'public'}}]};
 assert.equal(exactSnapshotMatches(point,context),true);
 for (const patch of [{matchId:'other'},{seat:1},{seq:2},{prefixHash:'fork'},{auditStateHash:'fork'},{publicStateHash:'fork'}]) assert.equal(exactSnapshotMatches({...point,...patch},context),false);
});
test('a bad persistent image falls back to genesis and must reproduce the signed head',async () => {
 const calls=[];
 const result=await recoverVerifiedRuntime({current:null,saved:[{level:'exact-build',seq:4}],
 restore:async()=>{calls.push('restore');throw Error('incompatible')}, genesis:async()=>calls.push('genesis'),
 replay:async seq=>calls.push(`replay:${seq}`),verify:async()=>calls.push('verify')});
 assert.equal(result.level,'genesis'); assert.deepEqual(calls,['restore','genesis','replay:0','verify']);
});
