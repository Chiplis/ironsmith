// Explicit native activation labels authorize references to the selected
// ability. A card-wide custom marker is never an activation-cost selector.
import {customKeywordForms} from './keyword-forms.js';
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
function activationLabel(definition){
 const reminder=String(definition.reminder||'');
 const labels=[...reminder.matchAll(/\bThis is the ([a-z][a-z0-9_-]*(?: [a-z][a-z0-9_-]*)*) activated ability\./gi)];
 if(labels.length!==1)return null;
 const before=reminder.slice(0,labels[0].index),colon=before.lastIndexOf(':');
 if(colon<0||before.slice(colon+1).trim()||((before.match(/["“”]/g)||[]).length%2))return null;
 return labels[0][1].toLowerCase();
}
export function expandNamedActivatedReferences(text,definition){
 const label=activationLabel(definition);if(!label)return String(text);
 const payments=[...String(definition.reminder||'').matchAll(/as payment named ["“]([a-z][a-z0-9_-]*(?: [a-z][a-z0-9_-]*)*)["”]\s*:/gi)];
 const payment=payments.length===1&&payments[0][1].toLowerCase()===label?label:null;
 const forms=customKeywordForms(definition),name=escape(forms.name);
 return String(text).split('\n').map(line=>{
  if(payment){
   const denial=new RegExp(`^((?:This|Enchanted|Equipped) (?:creature|permanent)) (can't attack and )?can't ${escape(forms.name)} (?:[a-z][a-z -]{0,60})[.]?$`,'i').exec(line.trim());
   if(denial){const restriction=`This creature can't be tapped to pay the generic tap payment named ${payment}.`;return (denial[2]?`${denial[1]} can't attack.\n`:'')+`${denial[1]} has "${restriction}".`;}
   const watcher=new RegExp(`^(\\s*(?:When|Whenever)) you ${escape(forms.name)} ((?:a|an|another) [a-z][a-z -]{0,60})(?=,)`,'i');
   const match=watcher.exec(line);
   if(match){const tail=line.slice(match[0].length).replace(/^(\s*,\s*(?:you may )?exile )that (?:card|permanent)\b/i,'$1the performer of this action');return `${match[1]} you perform the ${payment} action from ${match[2]}`+tail;}
  }
  const comma=line.indexOf(','),head=comma<0?'':line.slice(0,comma);
  const trigger=/^\s*(?:When|Whenever)\b/i.test(head);
  return line.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index)=>{
   if(index%2)return part;
   const cost=new RegExp(`\\b${name} abilities of ([^.;\\n]+?) cost ((?:\\{[^{}]+\\})+) less(?: to activate)?(?=\\s*[.;]|$)`,'gi');
   part=part.replace(cost,(_,objects,mana)=>`Activated abilities named "${label}" of ${objects.trim()} cost ${mana} less to activate`);
   if(trigger&&forms.eventVerb){
    const verb=escape(forms.eventVerb),event=String(forms.eventLabel||definition.keyword).toLowerCase();
    const occurrence=new RegExp(`\\b(you|a player|an opponent)\\s+${verb}s?\\s+((?:a|an|another)\\s+(?:(?:creature|artifact|enchantment|land|permanent)\\s+)?token)\\b`,'gi');
    const split=part.indexOf(',');
    if(split>=0)part=part.slice(0,split).replace(occurrence,(_,actor,objects)=>`${objects} participates in the ${event} action performed by ${actor.toLowerCase()}`)+part.slice(split);
   }
   return part;
  }).join('');
 }).join('\n');
}
export const hasNamedActivatedReference=(text,definition)=>expandNamedActivatedReferences(text,definition)!==String(text);
