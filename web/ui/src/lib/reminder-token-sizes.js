// A declaration supplies a list of token sizes; the definition supplies their
// shared blueprint and the surrounding ability program.
import { bindReminderValueFromDeclaration } from './reminder-costs.js';
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
export const hasTokenSizeParameters=definition=>/\{TokenSizes\}/i.test(String(definition.template||definition.reminder||''));
export function validateTokenSizeTemplate(keyword,template,reminder) {
  if(!/^[\s—–-]*\{TokenSizes\}\s*$/i.test(String(template)))throw new Error(`"${keyword}" needs one {TokenSizes} declaration operand.`);
  if((String(reminder).match(/\{TokenSizes\}/gi)||[]).length!==1 || !/create\s+\{TokenSizes\}\s+[^.]+\s+creature tokens?\b/i.test(reminder))throw new Error(`"${keyword}" must place {TokenSizes} in one complete creature-token creation instruction.`);
}
function sizes(value) {
 const items=String(value).replace(/,\s*where\s+[XYZ]\s+is\s+[^.!?]+\.?\s*$/i,'').trim().split(/\s*,\s*(?:and\s+)?|\s+and\s+/i);
 if(!items.length)return null;
 const result=[];
 for(const item of items){const match=item.match(/^(?:([0-9]+|[XYZ])\s+)?([+-]?\d+)\/([+-]?\d+)(?:s)?$/i);if(!match)return null;result.push({count:match[1]||'a',power:match[2],toughness:match[3]});}
 return result;
}
export function expandTokenSizeParameters(text,definition) {
 validateTokenSizeTemplate(definition.keyword,definition.template,definition.reminder);
 const expression=new RegExp(`(^|\\n)([ \\t]*)${escape(definition.keyword)}\\s*[—–-]?\\s+([^\\n]+)`, 'gi');
 return String(text).replace(expression,(whole,start,indent,declaration)=>{
  const operands=sizes(declaration);if(!operands)throw new Error(`"${definition.keyword}" needs a complete list of token sizes.`);
  const slot=definition.reminder.match(/\bcreate\s+\{TokenSizes\}\s+/i);
  const blueprintStart=slot.index+slot[0].length;let end=blueprintStart,quoted=false;
  for(;end<definition.reminder.length;end++){const character=definition.reminder[end];if(/["“”]/.test(character))quoted=!quoted;if(character==='.'&&!quoted)break;}
  const blueprint=definition.reminder.slice(blueprintStart,end);
  let reminder=definition.reminder.slice(0,slot.index)+'create '+operands.map(({count,power,toughness})=>`${count} ${power}/${toughness} ${count==='a'?blueprint.replace(/\bcreature tokens\b/i,'creature token'):blueprint}`).join(' and ')+definition.reminder.slice(end);
  reminder=bindReminderValueFromDeclaration(reminder,declaration).text;
  return start+indent+reminder;
 });
}

// Group noun references derive their meaning from the independently supplied
// grouping program. Printed keyword declarations still bind their own operand.
export function expandDeclaredGroupReferences(text,definitions,sourceName) {
 let result=String(text);
 for(const definition of definitions){
  if(!hasTokenSizeParameters(definition)||! /\bGroup\b[^.]+\bso all members attack and block if any member or the owner does, if able\b/i.test(definition.reminder))continue;
  const noun=escape(definition.keyword),owner=`(?:${[sourceName,'this creature','this permanent','that creature','target creature'].filter(Boolean).map(escape).join('|')})`;
  const possessive=`(${owner})['’]s?\\s+${noun}`;
  result=result.replace(new RegExp(`\\bput\\s+([^.!?]+?)\\s+(?:in|into)\\s+${possessive}`,'gi'),(_,subjects,reference)=>`Group ${subjects} under ${reference} so all members attack and block if any member or the owner does, if able`);
  result=result.replace(new RegExp(`\\bput\\s+([^.!?]+?)\\s+(?:in|into)\\s+(?:the\\s+)?same\\s+${noun}`,'gi'),(_,subjects)=>`Group ${subjects} under this creature so all members attack and block if any member or the owner does, if able`);
  result=result.replace(new RegExp(`\\bput\\s+([^.!?]+?)\\s+(?:in|into)\\s+a\\s+${noun}`,'gi'),(_,subjects)=>`Group ${subjects} so all members attack and block if any member does, if able`);
  result=result.replace(new RegExp(`\\bin\\s+${possessive}`,'gi'),(_,reference)=>`in group owned by ${reference}`);
  result=result.replace(new RegExp(`\\bin\\s+its\\s+${noun}`,'gi'),'in group owned by it');
 }
 return result.replace(/\band\s+Group\b/g,'. Group');
}
