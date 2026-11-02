import { baseAssetUrl } from './asset-base.js';
let catalog;
const loaded = new Map();
async function readAsset(path) {
  const response = await fetch(new URL(path, baseAssetUrl()));
  if (!response.ok) throw new Error(`Could not load bundled custom sets (${response.status})`);
  return response.json();
}
export async function loadBundledSetCatalog() {
  if (!catalog) catalog = readAsset('custom-sets/index.json').catch(error => { catalog = null; throw error; });
  return catalog;
}
export async function resolveBundledSet(entry) {
  if (!entry?.bundlePath) return structuredClone(entry);
  if (!loaded.has(entry.id)) loaded.set(entry.id, readAsset(entry.bundlePath).catch(error => { loaded.delete(entry.id); throw error; }));
  const set = structuredClone(await loaded.get(entry.id));
  return structuredClone(set);
}
export function mergeBundledLibrary(saved, entries) {
  // Saving an edited bundled set shadows the shipped version only in this browser.
  const names = new Set(saved.sets.map(set => set.name.toLowerCase()));
  return {...saved, sets:[...saved.sets, ...entries.filter(set=>!names.has(set.name.toLowerCase()))]};
}

let searchIndex;
export async function searchBundledCustomCards(query, limit = 5) {
  const needle=String(query || '').trim().toLowerCase();
  if(!needle)return [];
  if(!searchIndex)searchIndex=readAsset('custom-sets/search.json').catch(error=>{searchIndex=null;throw error;});
  const [index,catalog]=await Promise.all([searchIndex,loadBundledSetCatalog()]);
  const matches=index.filter(row=>row.names.some(name=>name.toLowerCase().includes(needle)))
    .sort((a,b)=>Number(b.names.some(n=>n.toLowerCase()===needle))-Number(a.names.some(n=>n.toLowerCase()===needle))).slice(0,limit);
  return Promise.all(matches.map(async row=>{
    const set=await resolveBundledSet(catalog.sets.find(set=>set.id===row.setId));
    return {name:row.names.join(' // '),card:set.cards.find(card=>card.id===row.id)};
  }));
}
