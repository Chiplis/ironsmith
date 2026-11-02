// A completed named additional tap payment is distinct from a permanent's status.
import {customKeywordForms} from './keyword-forms.js';
import {suggestSubtypePlural} from './custom-subtypes.js';
const escape=value=>value.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
// Irregular subtype surfaces also accepted by the native leaf type grammar.
const outsideQuotes=(text,fn)=>text.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index)=>index%2?part:fn(part)).join('');
const irregular={fungus:'fungi',mouse:'mice',ox:'oxen',ouphe:'ouphe',spacecraft:'spacecraft'};
function typeForms(type){const key=type.toLowerCase();return new Set([key,suggestSubtypePlural(key).toLowerCase(),irregular[key]].filter(Boolean));}
export function bindNamedTapPaymentDefinition(definition){
 const body=String(definition.expansion||definition.reminder||'');
 if(!/^As an additional cost to cast this spell, you may tap an untapped [a-z '-]+ you control\. This spell costs \{X\} less to cast, where X is the power of the creature tapped this way\.?$/i.test(body))return definition;
 const bind=value=>String(value).replace(/^As an additional cost to cast this spell,/i,`As an additional cost named "${definition.keyword}" to cast this spell,`);
 return {...definition,expansion:bind(body)};
}
export function expandTapPaymentReferences(text,definitions,faceName=''){
 let result=String(text);
 for(const definition of definitions){
  const reminder=String(definition.expansion||definition.reminder||'');
  const payment=reminder.match(/^As an additional cost(?: named ["'][^"']+["'])? to cast this spell, you may tap an untapped ([a-z '-]+) you control\. This spell costs \{X\} less to cast, where X is the power of the creature tapped this way\.?$/i);
  if(!payment)continue;
  const forms=customKeywordForms(definition),past=escape(forms.participle),label=definition.keyword;
  const receipt=`the selected ${label} payment object`;
  // Active donor language reads the completed cost receipt directly. The
  // selected-object tag exists only after an explicit later selection.
  const donor=`the creature tapped to pay its ${label} additional cost`;
  const pastForms=[forms.participle,definition.sourceKeyword ? customKeywordForms({keyword:definition.sourceKeyword}).participle : null].filter(Boolean).map(escape).join('|');
  const verbs=[label,definition.sourceKeyword].filter(Boolean).map(escape).join('|');
  result=outsideQuotes(result,part=>part
    .replace(new RegExp(`\\b(?:the )?creature that (?:${pastForms}) (?:it|this spell)\\b`,'gi'),donor)
    .replace(new RegExp(`\\bthat (?:did not|didn't|didn’t) (?:${verbs}) (?:it|this spell)\\b`,'gi'),`that was not tapped to pay its ${label} additional cost`));
  // Only an exact redundant domain restriction can disappear: the complete
  // native payment already restricts the same subtype, including its plural.
  const names=[label,definition.sourceKeyword].filter(Boolean).map(escape).join('|');
  const qualifier=new RegExp(`Only ([a-z '-]+) can be tapped to (?:${names}) those spells\\.`,'gi');
  result=outsideQuotes(result,part=>part.replace(qualifier,(original,type)=>typeForms(payment[1]).has(type.trim().toLowerCase())?'':original));
  const source=['it','this spell','this permanent','this creature','this artifact',faceName].filter(Boolean).map(escape).join('|');
  result=result.split('\n').map(line=>{
   if(/["“”]/.test(line))return line;
   // This source's self-entry watcher owns the receipt and its subsequent noun.
   const trigger=new RegExp(`^((?:When|Whenever) (?:${source}) enters,\\s*)if (?:${source}) was ${past} by (an? [a-z '-]+),([\\s\\S]*)$`,'i').exec(line);
   if(trigger){const [,head,filter,body]=trigger;const noun=filter.replace(/^an?\s+/i,'');
    const bound=body.replace(new RegExp(`\\bthat ${escape(noun)}\\b`,'gi'),receipt).replace(/\bto it\b/gi,`to ${receipt}`);
    return `${head}if ${filter} was tapped to pay its ${label} additional cost, choose ${filter} from the creatures tapped to pay its ${label} additional cost. ${bound.trim()}`;
   }
   const paid=new RegExp(`\\bif this spell was (?:${pastForms})\\b`,'gi');return line.replace(paid,`if this spell paid the additional cost named ${label}`);
  }).join('\n');
 }
 return result;
}
