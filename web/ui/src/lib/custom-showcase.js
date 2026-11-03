import { baseAssetUrl } from './asset-base.js';
import { populateCustomShowcase } from './custom-showcase-setup.js';
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
// The pool was expanded and checked at build time. Startup only samples and
// deals it; the full editable source sets remain lazy-loaded by the editor.
export async function loadCustomShowcase(game, { random=Math.random, players=['You','Opponent'], librarySize=30,
  onProgress=()=>{}, poolLoader=preloadCustomShowcase } = {}) {
  const started=performance.now();
  const pool=await poolLoader();
  const wanted=players.length*(librarySize+7);
  const queues=['msem','rev'].map(scope=>shuffled(pool.cards.filter(value=>value.card.draft.attribution.sourceFormat===scope),random));
  const prepared=[];
  while(prepared.length<wanted && queues.some(queue=>queue.length)) {
    for(const queue of queues)if(queue.length && prepared.length<wanted)prepared.push(queue.pop());
  }
  if(prepared.length<wanted)throw Error('The bundled showcase pool is too small. Rebuild it before deploying.');
  onProgress({ready:wanted,total:wanted});
  const plan={players,librarySize,support:pool.support,prepared:prepared.map(value=>({input:value.input,source:value.source,artifacts:value.artifacts})),fixer:pool.fixer};
  const dealStarted=performance.now();
  const setup=game.populateCustomShowcase ? await game.populateCustomShowcase(plan) : await populateCustomShowcase(game,plan);
  performance.measure('custom-showcase-deal',{start:dealStarted,detail:setup});
  performance.measure('custom-showcase-startup',{start:started});
  registerCustomCardAssets(prepared.map(value=>({...value.card,draft:{...value.card.draft,customMechanics:value.input.customMechanics || value.card.draft.customMechanics}})));
  registerCustomCardAssets([{draft:{...MANA_FIXER_DRAFT,attribution:{}}}]);
  setCompiledCardNames(['IronSmith Mana Fixer',...prepared.flatMap(value=>value.card.draft.faces.map(face=>face.name))]);
  return {cards:wanted,skipped:[]};
}

let poolPromise;
export function preloadCustomShowcase() {
 if(!poolPromise)poolPromise=loadShowcasePool().catch(error=>{poolPromise=null;throw error;});
 return poolPromise;
}
async function loadShowcasePool() {
 const response=await fetch(new URL('custom-sets/showcase.json',baseAssetUrl()));
 if(!response.ok)throw Error(`Could not load the bundled custom showcase (${response.status})`);
 const pool=await response.json();
 if(pool.version!==1 || !Array.isArray(pool.cards) || !pool.fixer?.artifacts?.length || !pool.support?.some(value=>value.source?.group?.name==='Omniscience') || pool.cards.some(value=>!value.artifacts?.length || !value.source))throw Error('Invalid bundled showcase pool');
 return pool;
}
