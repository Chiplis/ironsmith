// Explicit custom-definition relation metadata scopes source-owned selectors.
// Labels and keyword forms are data; no card or mechanic name is recognized.
import { customKeywordForms } from './keyword-forms.js';
const escape=value=>value.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
const zoneConditions={battlefield:'this card is on the battlefield',stack:'this card is on the stack',exile:'this card is in exile',hand:'this card is in your hand',graveyard:'this card is in your graveyard',library:'this card is in your library',command:'this card is in the command zone'};
export function normalizeObjectRelation(value){
 if(value==null)return null;
 if(typeof value!=='object'||Array.isArray(value)||typeof value.name!=='string'||! /^[a-z][a-z -]{0,63}$/.test(value.name)||!Object.hasOwn(zoneConditions,value.sourceZone)||!Object.hasOwn(zoneConditions,value.memberZone)||Object.keys(value).some(key=>!['name','sourceZone','memberZone'].includes(key)))throw new Error('Object relation requires a lowercase collection name and explicit source/member zones.');
 return {name:value.name,sourceZone:value.sourceZone,memberZone:value.memberZone};
}
export function expandSourceObjectRelationReferences(text,definitions){
 let result=String(text);
 for(const def of definitions){
  const relation=normalizeObjectRelation(def.objectRelation);if(!relation||relation.memberZone!=='battlefield')continue;
  const term=escape(customKeywordForms(def).participle);const condition=zoneConditions[relation.sourceZone];
  result=result.split('\n').map(line=>{
   // A plain static relation subject owns only its bounded prefix. Preserve
   // quoted child abilities in the body verbatim rather than inspecting them.
   const staticSubject=line.match(new RegExp(`^${term} (creature|permanent) (gets? [\\s\\S]+|has [\\s\\S]+)[.]$`,'i'));
   if(staticSubject)return `As long as ${condition}, ${staticSubject[1]} from the ${relation.name} collection ${staticSubject[2]}.`;
   if(/["“”]/.test(line))return line;
   // Source-relative Equipment attachment and a named exact relation have
   // independent source zones, so each ordinary static rule owns its scope.
   let match=line.match(new RegExp(`^Equipped or ${term} (creature|permanent) (gets? [^.!?]+[.]|has [^.!?]+[.])$`,'i'));
   if(match)return `Equipped ${match[1]} ${match[2]}\nAs long as ${condition}, ${match[1]} from the ${relation.name} collection ${match[2]}`;
   match=line.match(new RegExp(`^(When|Whenever) ${term} (creature|permanent) dies, (.+)$`,'i'));
   if(match)return `${match[1]} a ${match[2]} from the ${relation.name} collection dies, if ${condition}, ${match[3]}`;
   match=line.match(new RegExp(`^((?:When|Whenever|At) [^,]+), ${term} (creature|permanent) (gets? [^.!?]+[.]|gains? [^.!?]+[.])$`,'i'));
   if(match)return `${match[1]}, if ${condition}, ${match[2]} from the ${relation.name} collection ${match[3]}`;
   return line;
  }).join('\n');
 }
 return result;
}
