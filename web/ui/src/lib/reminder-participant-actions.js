// A named operation with affected objects defines the event denoted by its
// participle. References read that event; they do not acquire the keyword.
import {customKeywordForms} from './keyword-forms.js';
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
export function expandParticipantActionReferences(text,definition){
 const before=String(text);
 const labels=[...new Set([...String(definition.expansion||definition.reminder).matchAll(/\bperform the ([a-z][a-z '-]*?) action with\b/gi)].map(match=>match[1].trim().toLowerCase()))];
 if(labels.length!==1)return before;
 const forms=customKeywordForms(definition);
 const participles=[...new Set([forms.participle,...forms.variants].filter(Boolean))].map(escape).join('|');
 if(!participles)return before;
 const pattern=new RegExp(`(^|[.!?\\n][ \\t]*)(When|Whenever)[ \\t]+([^,\\n.!?]+?)[ \\t]+(becomes?)[ \\t]+(?:${participles})(?:[ \\t]+to[ \\t]+([^,\\n.!?]+?))?(?=[ \\t]*,)`,'gim');
 const rewritten=before.replace(pattern,(whole,prefix,when,subject,verb,performer,offset)=>outsideQuotation(before,offset+prefix.length)?`${prefix}${when} ${subject} ${/^becomes$/i.test(verb)?'participates':'participate'} in the ${labels[0]} action${performer?` performed by ${performer.trim()}`:''}`:whole);
 const header=new RegExp(`^\\s*(?:When|Whenever)\\b[^,\\n]*\\bparticipates? in the ${escape(labels[0])} action(?: performed by [^,\\n]+)?,`,'i');
 const relative=new RegExp(`\\bthe (?:creature|permanent|artifact|enchantment|land|card|object) it ${escape(forms.participle)}\\b`,'gi');
 return rewritten.split('\n').map(line=>{
  if(!header.test(line))return line;
  return line.replace(relative,(match,offset)=>outsideQuotation(line,offset)?'the performer of this action':match);
 }).join('\n');
}
export const hasParticipantActionReference=(text,definition)=>expandParticipantActionReferences(text,definition)!==String(text);
