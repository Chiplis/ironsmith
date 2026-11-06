// Object action identities come from explicit executable atoms, independently
// of a keyword's spelling. Event heads and numeric references keep their own
// source/filter/amount operands; consequence paragraphs stay with the compiler.
import {customKeywordForms} from './keyword-forms.js';
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
const outsideQuotation=(text,offset)=>(text.slice(0,offset).match(/["“”]/g)||[]).length%2===0;
function relation(definition) {
 const labels=[...new Set([...String(definition.reminder||'').matchAll(/Have this permanent perform the ([a-z][a-z -]*?) action\./gi)].map(m=>m[1].toLowerCase()))];
 if(labels.length!==1)return null;
 const forms=customKeywordForms(definition);
 const head=forms.name.endsWith('s')?`${forms.name}es`:forms.name.endsWith('y')&&!/[aeiou]y$/.test(forms.name)?`${forms.name.slice(0,-1)}ies`:`${forms.name}s`;
 return {label:labels[0],head,participle:forms.participle};
}
export function expandNamedActionState(text,definition,sourceName='') {
 const action=relation(definition);if(!action)return String(text);
 const original=String(text), source=sourceName?`${escape(sourceName)}|`:'';
 const own=`(?:${source}this (?:creature|permanent|artifact|enchantment|land|planeswalker|battle)|it)`;
 let result=original.replace(new RegExp(`(^|\\n)((?:When|Whenever)\\s+)(${own})\\s+(enters\\s+or\\s+)?${escape(action.head)}(?=,)`,'gi'),(whole,line,intro,subject,enters,offset)=>{
  if(!outsideQuotation(original,offset))return whole;
  return `${line}${intro}${enters?'this permanent enters or ':''}this permanent performs the ${action.label} action`;
 });
 result=result.replace(new RegExp(`(\\b(?:When|Whenever)\\s+|\\bor\\s+)((?:a|an|another) [^,\\n]+?)\\s+${escape(action.head)}(?=,)`,'gi'),(whole,prefix,subject,offset)=>outsideQuotation(result,offset)?`${prefix}${subject} performs the ${action.label} action`:whole);
 if(sourceName){
  result=result.replace(new RegExp(`the number of times (it|this permanent) and ${escape(sourceName)} have ${escape(action.participle)}( this turn)?`,'gi'),(_,subject,turn)=>`the number of times ${subject} has performed the ${action.label} action${turn||''} plus the number of times this permanent has performed the ${action.label} action${turn||''}`);
 }
 result=result.replace(new RegExp(`the number of times (${own}) has ${escape(action.participle)}( this turn)?`,'gi'),(_,subject,turn)=>`the number of times ${/^it$/i.test(subject)?'it':'this permanent'} has performed the ${action.label} action${turn||''}`);
 result=result.replace(new RegExp(`((?:\\{[^{}]+\\})+) was spent to ${escape(customKeywordForms(definition).name)} (?:it|this permanent)`,'gi'),(_,mana)=>`${mana} was spent to perform the ${action.label} action`);
 return result;
}
export const hasNamedActionState=(text,definition,sourceName='')=>expandNamedActionState(text,definition,sourceName)!==String(text);
