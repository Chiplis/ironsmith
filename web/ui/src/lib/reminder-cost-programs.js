// An explicitly declared effect program may be paid as one bounded cost.
import { parseParameterDeclaration, bindReminderParameters } from './reminder-costs.js';
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
const outsideQuotation=(text,offset)=>(text.slice(0,offset).match(/["“”]/g)||[]).length%2===0;
export function expandCostProgramDeclarations(text,definition){
 if(!definition.costProgram)return text;
 const before=expandResolvingReceiptPrograms(expandLocalChoiceCountPrograms(String(text),definition),definition);
 const pattern=new RegExp(`\\b${escape(definition.keyword)}[ \\t]+(\\d+|one|two|three|four|five|six|seven|eight|nine|ten|X)(?=[ \\t]*[:,.;]|$)`,'gi');
 return before.replace(pattern,(original,_amount,offset)=>{
  if(!outsideQuotation(before,offset))return original;
  const start=before.lastIndexOf('\n',offset-1)+1;
  const prefix=before.slice(start,offset);
  const after=before.slice(offset+original.length).split('\n')[0];
  const additional=/^As an additional cost to cast this spell,[ \t]*(?:you[ \t]+may)?$/i.test(prefix.trim());
  const activation=!prefix.includes(':')&&/^[ \t]*(?:,[^:.]*)?:/.test(after);
  const resolvingOptional=/\byou[ \t]+may[ \t]*$/i.test(prefix);
  if(!additional&&!activation&&!resolvingOptional)return original;
  if(/^X$/i.test(_amount)&&!additional&&!activation)return original;
  const values=parseParameterDeclaration(definition.reminder,definition.keyword,original,definition.template);
  if(!values)return original;
  const program=bindReminderParameters(definition.expansion,definition.keyword,values).trim().replace(/[.!?]+$/,'').replace(/[.!?]/g,';');
  return `Pay the cost "${program}"`;
 });
}
export function costProgramReferences(text,definition){
 if(!definition.costProgram)return text;
 let result=String(text);
 const verb=definition.forms?.eventVerb,label=definition.forms?.eventLabel;
 if(verb&&label)for(const reference of definition.paymentReferences||[]){
  if(!reference.countUnit)continue;
  const pattern=new RegExp(`^(\\s*)(Whenever|When)[ \\t]+you[ \\t]+${escape(verb)}[ \\t]+from[ \\t]+(\\d+|one|two|three|four|five|six|seven|eight|nine|ten)[ \\t]+or[ \\t]+more[ \\t]+${escape(reference.countUnit)}s([ \\t]+while[^,\\n]*)?[ \\t]*,([^\\n]*)$`,'gim');
  result=result.replace(pattern,(_,space,intro,minimum,qualifier,body)=>{
   const words=['zero','one','two','three','four','five','six','seven','eight','nine','ten'];
   const amount=/^\d+$/.test(minimum)?Number(minimum):words.indexOf(minimum.toLowerCase());
   return `${space}${intro} you perform the ${label} action with an amount of ${amount} or more${qualifier||''},${body.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index)=>index%2?part:part.replace(/\bthat number(?:[ \t]+of)?\b/gi,'that many')).join('')}`;
  });
 }
 const additional=[...result.matchAll(/As an additional cost to cast this spell, you may pay the cost "[^"\n]+"/gi)];
 if(additional.length!==1)return result;
 const optional=result.match(/As an additional cost to cast this spell, you may\b/gi)||[];
 if(optional.length!==1)return result;
 for(const reference of definition.paymentReferences||[]){
  const pattern=new RegExp(`\\b${escape(reference.subject)}[ \\t]+(was|wasn't|was not)[ \\t]+${escape(reference.participle)}\\b`,'gi');
  result=result.replace(pattern,(match,verb,offset)=>outsideQuotation(result,offset)?`this spell's additional cost ${/^was$/i.test(verb)?'was':"wasn't"} paid`:match);
 }
 return result;
}

// Resolving variable actions choose their local amount. The explicit bounded
// source scope pins X to this payment's historical chosen collection before
// any subsequent choice can publish another collection or rebind X.
export function expandLocalChoiceCountPrograms(text,definition){
 if(!/^Choose \{Number\}\s/i.test(String(definition.expansion)))return text;
 const pattern=new RegExp(`\\byou[ \\t]+may[ \\t]+${escape(definition.keyword)}[ \\t]+X[.][ \\t]*(.*)$`,'gim');
 return String(text).replace(pattern,(original,tail,offset)=>{
  if(!outsideQuotation(String(text),offset))return original;
  if(tail.trim()&&!/^(?:If you do|When you do)\b/i.test(tail))return original;
  const values={number:'any number of'};
  const program=bindReminderParameters(definition.expansion,definition.keyword,values).trim().replace(/[.!?]+$/,'').replace(/[.!?]/g,';');
  const body=tail.trim().replace(/[.!?]+$/,'').split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index)=>index%2?part:part.replace(/[.!?]/g,';')).join('');
  const payment=resolvedPaymentProgram(definition,program);
  return tail.trim() ? `${payment}. With X equal to the number of objects chosen this way, do the following: ${body}` : `${payment}.`;
 });
}

function resolvedPaymentProgram(definition,program){
 const payment=`You may pay the cost "${program}"`;
 const label=definition.forms?.eventLabel;
 return label ? `Perform the ${label} action with objects chosen this way if the following cost is paid: ${payment}` : payment;
}
function expandResolvingReceiptPrograms(text,definition){
 if(!definition.forms?.eventLabel)return text;
 const pattern=new RegExp(`\\byou[ \\t]+may[ \\t]+${escape(definition.keyword)}[ \\t]+(\\d+|one|two|three|four|five|six|seven|eight|nine|ten)(?=[ \\t]*[.;]|$)`,'gi');
 return String(text).replace(pattern,(original,amount,offset)=>{
  if(!outsideQuotation(String(text),offset))return original;
  const start=String(text).lastIndexOf('\n',offset-1)+1;
  if(/As an additional cost to cast/i.test(String(text).slice(start,offset)))return original;
  const values=parseParameterDeclaration(definition.reminder,definition.keyword,`${definition.keyword} ${amount}`,definition.template);
  if(!values)return original;
  const program=bindReminderParameters(definition.expansion,definition.keyword,values).trim().replace(/[.!?]+$/,'').replace(/[.!?]/g,';');
  return resolvedPaymentProgram(definition,program);
 });
}

// Admission uses the same bounded reader as expansion, so declarations in
// optional prices and resolving clauses cannot be dropped before expansion.
export const hasCostProgramDeclaration = (text,definition) => Boolean(definition.costProgram)
 && expandCostProgramDeclarations(String(text),definition) !== String(text);
export const hasCostProgramReference = (text,definition) => Boolean(definition.costProgram)
 && costProgramReferences(String(text),definition) !== String(text);
