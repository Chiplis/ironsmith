import { normalizeDraftForApi } from './custom-card-draft.js';
import { loadBundledSetCatalog, resolveBundledSet } from './bundled-custom-sets.js';
import { prepareCustomCard } from './custom-mechanics.js';
import { setCompiledCardNames } from './scryfall.js';
import { registerCustomCardAssets } from './custom-card-assets.js';

export const MANA_FIXER_DRAFT = {
  layout:'single', faces:[{name:'IronSmith Mana Fixer',manaCost:'{0}',supertypes:[],cardTypes:['Artifact'],subtypes:[],
    oracleText:'{0}: Add {W}{U}{B}{R}{G}.',power:'',toughness:'',loyalty:'',defense:'',colorIndicator:[]}],
};
export function isCustomShowcase(search = globalThis.location?.search || '') {
  return new URLSearchParams(search).get('showcase') === 'custom';
}
function shuffled(values, random) {
  const copy=[...values];
  for(let i=copy.length-1;i>0;i--){const j=Math.floor(random()*(i+1));[copy[i],copy[j]]=[copy[j],copy[i]];}
  return copy;
}
// Prepare the complete table before replacing the current game. A failure
// restores the old table, rather than leaving a half-filled showcase.
export async function loadCustomShowcase(game, { random=Math.random, players=['You','Opponent'], librarySize=30,
  catalogLoader=loadBundledSetCatalog, setLoader=resolveBundledSet, prepare=prepareCustomCard, onProgress=()=>{} } = {}) {
  const catalog=await catalogLoader();
  const wanted=players.length*(librarySize+7), prepared=[], names=new Set(), skipped=[];
  const sets=shuffled(catalog.sets,random);
  // Alternate the two formats so a small table always showcases both.
  const queues=['msem','rev'].map(scope=>sets.filter(set=>set.source.scope===scope));
  const ordered=[];
  while(queues.some(q=>q.length)) for(const queue of queues)if(queue.length)ordered.push(queue.pop());
  for(const entry of ordered) {
    const set=await setLoader(entry);
    let accepted=0;
    for(const card of shuffled(set.cards,random)) {
      if(prepared.length>=wanted || accepted>=8) break;
      if(card.importIssue || card.showcaseExcluded || card.draft.token) continue;
      const identity=card.draft.faces.map(face=>face.name).join(' // ');
      if(names.has(identity))continue;
      try {
        const result=await prepare(normalizeDraftForApi(card.draft),input=>game.previewCustomCard(input));
        prepared.push({card,input:result.input});names.add(identity);accepted++;onProgress({ready:prepared.length,total:wanted});
      } catch(error) {skipped.push({id:card.id,reason:String(error.message||error)});}
      if(skipped.length>=250)break;
    }
    if(prepared.length>=wanted || skipped.length>=250)break;
  }
  if(prepared.length<wanted) {
    console.error('Custom showcase compilation failures', skipped);
    throw new Error(`The custom showcase needs ${wanted} compilable cards; found ${prepared.length}.`);
  }
  const fixer=await prepare(normalizeDraftForApi(MANA_FIXER_DRAFT),input=>game.previewCustomCard(input));
  const checkpoint=await game.createRuntimeSavepoint();
  try {
    await game.resetEmpty(players,20);
    let cursor=0;
    for(let playerIndex=0;playerIndex<players.length;playerIndex++) {
      await game.addCardToZone(playerIndex,'Omniscience','battlefield',true);
      await game.createCustomCard({draft:fixer.input,playerIndex,zoneName:'battlefield',skipTriggers:true});
      for(let i=0;i<librarySize+7;i++) {
        const value=prepared[cursor++];
        await game.createCustomCard({draft:value.input,playerIndex,zoneName:i<7?'hand':'library',skipTriggers:true});
      }
    }
    await game.finishPuzzleSetup();
    registerCustomCardAssets(prepared.map(value=>value.card));
    setCompiledCardNames([...names,'IronSmith Mana Fixer',...prepared.flatMap(value=>value.card.draft.faces.map(face=>face.name))]);
    return {cards:prepared.length,skipped};
  } catch(error) { await game.restoreRuntimeSavepoint(checkpoint); throw error; }
  finally { await game.releaseRuntimeSavepoint(checkpoint); }
}
