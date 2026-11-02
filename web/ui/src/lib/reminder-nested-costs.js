// Cost syntax can contain an independently authored keyword action. Resolve
// only a whole declaration's price operand, never free prose or an effect.
import { reminderParameters, parseParameterDeclaration, bindReminderParameters } from './reminder-costs.js';
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
const costAction = text => /^(?:pay|exile|discard|sacrifice|return|tap|remove)\b/i.test(text)
  && !/[.!?\n]/.test(text) && !/\b(?:then|if|unless|until|whenever|when|draw|create|destroy)\b/i.test(text);
export function expandNestedCostPrograms(text, definitions) {
 const before=String(text);
 let result=before;
 for(const outer of definitions){
  const parameters=reminderParameters(outer.reminder);
  if(parameters.length!==1||parameters[0]!=='cost'||(outer.template&&outer.template!=='{Cost}'))continue;
  const pattern=new RegExp(`(^[ \\t]*${escape(outer.keyword)}[ \\t]*[—–-]?[ \\t]*)([^\\n]+)$`,'gim');
  result=result.replace(pattern,(whole,prefix,operand,offset)=>{
   if(!outsideQuotation(result,offset))return whole;
   const candidates=[];
   let recognized=false;
   for(const inner of definitions){
    if(inner===outer)continue;
    const declared=new RegExp(`^${escape(inner.keyword)}(?![a-z])`,'i');
    if(!declared.test(operand.trim()))continue;
    recognized=true;
    const values=parseParameterDeclaration(inner.reminder,inner.keyword,operand,inner.template);
    if(!values)continue;
    const action=bindReminderParameters(inner.expansion,inner.keyword,values).trim().replace(/[.!?]+$/,'');
    if(costAction(action))candidates.push(action);
   }
   if(recognized&&candidates.length!==1)throw new Error('A nested cost must bind exactly one independent payment action.');
   return candidates.length===1?prefix+candidates[0]:whole;
  });
 }
 return result;
}
