// Source menu labels are operands, never intrinsic meanings of counter names.
import {parseParameterDeclaration,bindReminderParameters} from './reminder-costs.js';
import {customKeywordForms} from './keyword-forms.js';
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
function counterMenuLabels(text,definitions) {
 const source=String(text),labels=[];
 for(const menu of definitions){
  if(!/\{counterKinds\}/i.test(menu.reminder)||menu.template?.toLowerCase()!=='{object}')continue;
  const pattern=new RegExp(`\\b${escape(menu.keyword)}(?:s)?\\s+[^.!?;\\n]+\\.\\s*\\n((?:[ \\t]*[•*-][ \\t]+[^\\n]+(?:\\n|$))+)`,'gi');
  for(const match of source.matchAll(pattern)){
   if((source.slice(0,match.index).match(/["“”]/g)||[]).length%2)continue;
   const offset=match.index+match[0].length-match[1].length;
   for(const line of match[1].matchAll(/^[ \t]*[•*-][ \t]+([^\n]+)(?:\n|$)/gm))labels.push({label:line[1].trim(),start:offset+line.index,end:offset+line.index+line[0].length});
  }
 }
 return labels.filter((entry,index)=>labels.findIndex(other=>other.start===entry.start&&other.end===entry.end)===index);
}
export function counterAbilityDefinitions(text,definitions) {
 const labels=counterMenuLabels(text,definitions).map(entry=>entry.label);
 const result=[];
 for(const label of labels){for(const def of definitions){
  let reminder;
  if(label.toLowerCase()===def.keyword.toLowerCase()&&!def.template){reminder=def.expansion;}
  else if(label.toLowerCase().startsWith(def.keyword.toLowerCase()+' ')){
   const values=parseParameterDeclaration(def.reminder,def.keyword,label,def.template);
   if(values)reminder=bindReminderParameters(def.expansion,def.keyword,values,customKeywordForms(def));
  }
  if(!reminder||/\{(?:cost|number|type|object|subject|ability|player|name|counterKinds)\}/i.test(reminder))continue;
  const name=label.toLowerCase();
  const previous=result.find(entry=>entry.name===name);
  if(previous&&previous.reminder.replace(/\s+/g,' ').trim()!==reminder.replace(/\s+/g,' ').trim())throw new Error(`Conflicting source-defined counter programs for "${name}".`);
  if(!previous)result.push({name,reminder});
 }}
 return result;
}
export function stripCounterAbilityLabels(text,counterDefinitions,definitions) {
 const source=String(text),ranges=counterMenuLabels(source,definitions).filter(entry=>counterDefinitions.some(def=>def.name===entry.label.toLowerCase()));
 let result=source;
 for(const range of ranges.sort((a,b)=>b.start-a.start))result=result.slice(0,range.start)+result.slice(range.end);
 return result;
}
