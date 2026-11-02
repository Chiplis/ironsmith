import {independentAbilityPrograms,unlabelledAbilityProgram} from './reminder-definition-programs.js';
import {quoteGrantedRuleProgram} from './reminder-granted-parameters.js';
import {hasReminderParameters} from './reminder-costs.js';
import {hasActionParameters} from './reminder-action-parameters.js';
const key=value=>String(value).trim().replace(/\s+/g,' ').toLowerCase();
const quoted=(text,index)=>(text.slice(0,index).match(/["“”]/g)||[]).length%2===1;
const source=(text,name)=>[name,'this permanent','this card','this creature','this land'].filter(Boolean).some(value=>key(value)===key(text));
const pattern=/(^|\n)(As long as ([^,\n]+?) with ([a-z][a-z -]*?) is exiled (by|with) ([^,\n]+), ([^,\n]+?) has ([a-z][a-z -]*?)\.)\s+The same is true for ([^.\n]+)\./gi;
const list=value=>value.split(/\s*,\s*(?:and\s+)?|\s+and\s+/i).map(item=>item.trim()).filter(Boolean);
const join=items=>items.length<2?items[0]:items.length===2?items.join(' and '):items.slice(0,-1).join(', ')+', and '+items.at(-1);
function known(def) {
 if(def.template||hasReminderParameters(def.reminder,def.template)||hasActionParameters(def))return false;
 const programs=independentAbilityPrograms(def.expansion||def.reminder);
 return programs.length>0 && programs.every(program=>quoteGrantedRuleProgram(program)
  && /^(?:When|Whenever|At|This (?:creature|permanent|spell)|(?:Un)?tapped creatures|Damage dealt to this (?:creature|permanent)|For each generic mana in this (?:spell|ability)'s total cost|You may cast)\b/i.test(unlabelledAbilityProgram(program)));
}
export function hasConditionalKeywordSharingReference(text,definition,sourceName) {
 return expandConditionalKeywordSharing(text,[definition],sourceName)!==String(text);
}
// Only a complete source-owned sharing statement and explicitly supplied custom
// names are expanded. The remaining ordinary/unknown list stays intact.
export function expandConditionalKeywordSharing(text,definitions,sourceName) {
 const before=String(text);
 return before.replace(pattern,(whole,prefix,head,subject,exemplar,relation,owner,recipient,granted,tail,offset)=>{
  if(quoted(before,offset)||key(exemplar)!==key(granted)||!source(owner,sourceName)
   || !(key(recipient)==='it'||source(recipient,sourceName)))return whole;
  const items=list(tail);
  const selected=items.map(item=>definitions.find(def=>key(def.keyword)===key(item)&&known(def))).filter(Boolean);
  if(!selected.length)return whole;
  const used=new Set(selected.map(def=>key(def.keyword)));
  const remaining=items.filter(item=>!used.has(key(item)));
  const ordinary=head+(remaining.length?' The same is true for '+join(remaining)+'.':'');
  const extra=[...new Map(selected.map(def=>[key(def.keyword),def])).values()].map(def=>{
   const bodies=independentAbilityPrograms(def.expansion||def.reminder).map(quoteGrantedRuleProgram);
   return `As long as ${subject} with custom keyword "${def.keyword}" is exiled ${relation} this permanent, this permanent has custom keyword "${def.keyword}", and ${bodies.join(', and ')}.`;
  });
  return prefix+[ordinary,...extra].join('\n');
 });
}
