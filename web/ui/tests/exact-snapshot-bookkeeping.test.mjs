import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const source=await readFile(new URL('../src/hooks/peer-lobby/crypto-resync.js',import.meta.url),'utf8');
const ref=value=>({current:value});
function harness() {
 const fields=['privateDeckManifests','ziffleKeyPairs','liveZiffleCeremonies','localZiffleCeremonyLookup','ziffleOpeningPositions','ziffleRevealTokenCache','localRevealedOpenings','privateViewDisclosures'];
 const context=Object.fromEntries(fields.map(name=>[`${name}Ref`,ref(new Map([[name,{secret:name}]]))]));
 Object.assign(context,{verifiedAuditOpeningsRef:ref(new Set(['opening'])),verifiedShuffleProofsRef:ref(new Set(['shuffle'])),
   writeStoredRevealedOpening(){},removeStoredRevealedOpening(){}, actionHistoryRef:ref([{seq:1}]),actionCursor:entries=>({entries,length:entries.length}),restoreActionCursor:cursor=>cursor.entries.slice(0,cursor.length),
   liveAuditTranscriptRef:ref({actions:[]}),matchStartPayloadRef:ref({genesis:{payloadHash:'signed'}}),auditStateHashRef:ref('accepted'),initialPublicCheckpointHashRef:ref('initial'),
   matchClockRef:ref({lastSequence:1}),matchClockConfigRef:ref({initialMs:60000}), actionCryptoRequirementsRef:ref(new Map([[1,['requirement']]])), relayedActionIdsRef:ref(new Set(['action'])),
   ziffleHandRevealKeyRef:ref('reveal'),ziffleHandRevealQuickKeyRef:ref('quick'),stateRef:ref({players:[]}),multiplayerRef:ref({lastAppliedSequence:1}),
   cloneMultiplayerPayload:structuredClone,isTrustedMultiplayerSecurityMode:()=>false,sessionSecurityMode:()=> 'verified',isMatchDisputed:()=>false,resolveLocalPlayerIndex:()=>0,
   setState:async()=>{},publishMatchClockSnapshot(){},runtimeMatchClockSnapshot:()=>({}),updateMultiplayer:fn=>context.multiplayerRef.current=fn(context.multiplayerRef.current)});
 let restores=0,releases=0;
 const disclosureRestores=[];
 context.servicesRef=ref({restorePaymentDisclosureAtHead:async anchor=>disclosureRestores.push(anchor)});
 const game={supportsRuntimeSavepoints:true,runtimeGeneration:0,createRuntimeSavepoint:async()=>1,restoreRuntimeSavepoint:async()=>restores++,copyRuntimeSavepoint:async()=>restores++,releaseRuntimeSavepoint:async()=>releases++,uiState:async()=>({players:[]})};
 context.gameRef=ref(game);
 const block=source.slice(source.indexOf('  async function createSequencedActionValidationSnapshot('),source.indexOf('\n\n  return { localRuntimeRecoveryCandidates'));
 const api=Function(...Object.keys(context),`${block}\nreturn {createSequencedActionValidationSnapshot,restoreSequencedActionValidationSnapshot};`)(...Object.values(context));
 return{context,game,api,restores:()=>restores,releases:()=>releases,disclosureRestores,fields};
}
test('persistent recovery restores transcript cursor, clock, requirements, private manifests, keys, openings and shuffle caches',async()=>{
 const {context:c,game,api,fields,restores,disclosureRestores}=harness();
 const point=await api.createSequencedActionValidationSnapshot();
 const {game:owner,release,runtimeHandle,runtimeGeneration,state,...data}=point;
 const persisted=structuredClone(data);
 for(const name of fields)c[`${name}Ref`].current.clear();
 c.verifiedAuditOpeningsRef.current.clear();c.verifiedShuffleProofsRef.current.clear();
 c.matchClockRef.current={lastSequence:2};c.actionHistoryRef.current.push({seq:2});
 game.runtimeGeneration++;
 await api.restoreSequencedActionValidationSnapshot({...persisted,game,runtimeGeneration:1},{runtimeAlreadyRestored:true});
 assert.equal(restores(),0,'restored instance must not consume an old native handle');
 assert.equal(c.actionHistoryRef.current.length,1);assert.equal(c.multiplayerRef.current.lastAppliedSequence,1);
 assert.equal(c.matchClockRef.current.lastSequence,1);assert.equal(c.auditStateHashRef.current,'accepted');
 assert.deepEqual(disclosureRestores,[{sequence:2,prevStateHash:'accepted'}]);
 for(const name of fields)assert.deepEqual([...c[`${name}Ref`].current],[[name,{secret:name}]]);
 assert.deepEqual([...c.verifiedAuditOpeningsRef.current],['opening']);assert.deepEqual([...c.verifiedShuffleProofsRef.current],['shuffle']);
 assert.deepEqual([...c.actionCryptoRequirementsRef.current],[[1,['requirement']]]);
});
test('old native rollback points cannot restore or release handles after instance replacement',async()=>{
 const {game,api,releases}=harness();const point=await api.createSequencedActionValidationSnapshot();game.runtimeGeneration++;
 await assert.rejects(api.restoreSequencedActionValidationSnapshot(point),/expired/);
 await point.release();assert.equal(releases(),0);
});

test('persistent recovery refuses rollback or forks after refresh, even when engine bytes are corrupt or a build changed', async()=>{
 const {sealExactSnapshot,validateExactSnapshotHeader,exactSnapshotMatches}=await import('../src/lib/exact-build-snapshot.js');
 const {assertResyncActionsExtendLocalTranscript}=await import('../src/lib/multiplayer-audit.js');
 const action={seq:1,actorIndex:0,command:{type:'pass'},prefixHash:'prefix',audit:{nextStateHash:'state',publicCheckpointHash:'public'}};
 const metadata={matchId:'match',seat:0,seq:1,prefixHash:'prefix',auditStateHash:'state',publicStateHash:'public',
  recoveryState:{lastAppliedSequence:1,auditStateHash:'state',actionHistoryCursor:{entries:[action],length:1}}};
 const image=await sealExactSnapshot({version:1,buildId:'build',pointer:8,memory:new Uint8Array(65536),globals:[],references:[],recovery:{metadata}});
 const gameRef=ref({supportsExactBuildSnapshots:true,exactSnapshotBuildId:'build'});
 let deleted=0;
 const context={gameRef, auditMatchInstanceId:()=> 'match',resolveLocalPlayerIndex:()=>0,multiplayerRef:ref({}),
  readExactSnapshot:async()=>({image}),validateExactSnapshotHeader,recordDiagnosticEvent(){},deleteExactSnapshot:async()=>deleted++,
  assertResyncActionsExtendLocalTranscript,restoreActionCursor:cursor=>cursor.entries.slice(0,cursor.length),exactSnapshotMatches,toErrorMessage:String};
 const block=source.slice(source.indexOf('  async function exactBuildRecoveryCandidates('),source.indexOf('  async function restoreExactBuildRecovery('));
 const candidates=Function(...Object.keys(context),`${block}\nreturn exactBuildRecoveryCandidates;`)(...Object.values(context));
 assert.equal((await candidates([action],{})).length,1);
 image.memory[0]^=1;
 await assert.rejects(candidates([],{}),/older than/);
 await assert.rejects(candidates([{...action,command:{type:'different'}}],{}),/does not match local/);
 gameRef.current.exactSnapshotBuildId='new-build';
 await assert.rejects(candidates([],{}),/older than/);
 assert.deepEqual(await candidates([action],{}),[]);
 assert.equal(deleted,1);
});
