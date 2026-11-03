// Run the prepared deal as one worker operation. No background legality worker
// is restarted or copied after each of the 78 individual placements.
export async function populateCustomShowcase(game, {players,librarySize,prepared,fixer,support=[]}) {
 const checkpoint=await game.createRuntimeSavepoint();
 try {
  await game.resetEmpty(players,20);
  const started=performance.now();
  for(const value of [...support,...prepared,fixer]) {
   if(!value.artifacts?.length || !value.source)throw Error('Showcase card is missing its compiled artifact. Rebuild the showcase before deploying.');
   try {await game.registerCompiledCardSourceArtifacts(value.source,value.artifacts);}
   catch(error) {throw Error(`Showcase artifact is incompatible. Rebuild the showcase for this engine: ${String(error)}`);}
  }
  const registered=performance.now();
  const placements=[];
  for(const zoneName of ['library','hand']) {
   for(let playerIndex=0;playerIndex<players.length;playerIndex++) {
    const offset=playerIndex*(librarySize+7);
    const start=zoneName==='library'?7:0, end=zoneName==='library'?librarySize+7:7;
    for(let i=start;i<end;i++)placements.push({value:prepared[offset+i],playerIndex,zoneName});
   }
  }
  for(let playerIndex=0;playerIndex<players.length;playerIndex++)placements.push({value:fixer,playerIndex,zoneName:'battlefield'});
  const batch=placements.map(({value,playerIndex,zoneName})=>({cardName:value.input.faces[0].name,playerIndex,zoneName,skipTriggers:true}));
  for(let playerIndex=0;playerIndex<players.length;playerIndex++)batch.push({cardName:'Omniscience',playerIndex,zoneName:'battlefield',skipTriggers:true});
  await game.stagePuzzleCardsToZones(batch);
  const staged=performance.now();
  await game.finishPuzzleSetup();
  return {baked:prepared.length+1,native:0,rejected:[],registrationMs:registered-started,placementMs:staged-registered,finishMs:performance.now()-staged};
 } catch(error) {await game.restoreRuntimeSavepoint(checkpoint);throw error;}
 finally {await game.releaseRuntimeSavepoint(checkpoint);}
}
