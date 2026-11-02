// Source-owned vocabulary is data. It never strips source types or supplies
// category abilities, and it never becomes a process-global parser registry.
import {readFileSync} from 'node:fs';
import {suppliedDefinitionFaces,mergeSubtypes,referencedSubtypes,subtypeFormsMentioned,canonicalSubtypeText} from '../../web/ui/src/lib/custom-subtypes.js';
const registry=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/source-subtypes.json',import.meta.url),'utf8'));
const scopeKey=scope=>scope==='revolution'?'rev':scope;
const vocabularyDraft=(card,draft)=>({...draft,faces:[...(draft.faces||[]), ...(draft.customTokenFaces||[]), ...suppliedDefinitionFaces([...(draft.suppliedCardDefinitions||[]),...(card.supplied_card_definitions||[])]),
 ...[...(card.custom_keyword_counter_definitions||[]),...(draft.customKeywordCounterDefinitions||[]),
 ...(draft.faces||[]).flatMap(face=>face.customKeywordCounterDefinitions||[])].map(item=>({oracleText:item.reminder||''}))]});
export function sourceSubtypeDefinitions(card,draft,scope) {
 draft=vocabularyDraft(card,draft);
 const explicit=card.source_custom_subtypes||card.customSubtypes||[];
 const text=(draft.faces||[]).flatMap(face=>[String(face.typeLine||'').split(/[—–]/).slice(1).join(' '),String(face.oracleText||'').replace(/\([^()]*\)/g,'')]).join('\n');
 const selected=new Map();
 const mentioned=word=>subtypeFormsMentioned(text,{singular:word,plural:word});
 for(const row of registry.definitions){
  if(row.scope!==scopeKey(scope)||(!mentioned(row.singular)&&!mentioned(row.plural)))continue;
  if(row.family==='Planeswalker'||row.family==='Planar'){
   const noun=row.family==='Planeswalker'?'planeswalkers?':'planes?';
   const explicitTypes=(draft.faces||[]).some(face=>{
    const line=String(face.typeLine||'');
    const head=line.split(/[—–]/)[0];
    return new RegExp(`\\b${noun}\\b`,'i').test(head)&&subtypeFormsMentioned(line.split(/[—–]/).slice(1).join(' '),row);
   });
   const typedReference=(draft.faces||[]).some(face=>new RegExp(`(?<![\\p{L}\\p{N}])(?:${row.singular}|${row.plural})[ \\t]+${noun}\\b|\\b${noun}[ \\t]+(?:named[ \\t]+)?(?:${row.singular}|${row.plural})(?![\\p{L}\\p{N}])`,'iu').test(canonicalSubtypeText(face.oracleText||'')));
   if(!explicitTypes&&!typedReference)continue;
  }
  const def={singular:row.singular,plural:row.plural,family:row.family};
  selected.set(`${def.singular}:${def.family}`,def);
 }
 // A source spelling denotes one shared subtype identity. Once selected,
 // transport every explicitly declared association, not a per-card family
 // guess that would make spell and permanent identities disagree.
 const names=new Set([...selected.values(),...explicit].map(def=>canonicalSubtypeText(def.singular)));
 const associations=registry.definitions.filter(row=>row.scope===scopeKey(scope)&&names.has(canonicalSubtypeText(row.singular)))
  .map(({singular,plural,family})=>({singular,plural,family}));
 return referencedSubtypes({...draft,customSubtypes:mergeSubtypes(explicit)},associations);
}
