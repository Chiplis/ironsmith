import { namedActionProgramDefinition } from './reminder-named-action-programs.js';
// A closed menu is an explicit operand, supplied by contiguous source bullets.
const escape=value=>value.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
export function expandCounterMenus(text,definitions,sourceName='') {
 let result=String(text);
 for(const def of definitions){
  if(!/\{counterKinds\}/i.test(def.reminder)||def.template?.toLowerCase()!=='{object}')continue;
  const pattern=new RegExp(`(?:\\b(you|that player)\\s+)?\\b${escape(def.keyword)}(?:s)?\\s+([^.!?;\\n]+)\\.(?:[ \t]+(When you (?:can't|cannot),[^\\n]+))?\\s*\\n((?:[ \\t]*[•*-][ \\t]+[^\\n]+(?:\\n|$))+)`,'gi');
  result=result.replace(pattern,(original,actor,object,unavailableFollowup,bullets,offset)=>{
   if((result.slice(0,offset).match(/["“”]/g)||[]).length%2)return original;
   actor=actor?.toLowerCase()||'';
   const recipients=object.trim().split(/\s+and\s+/i);
   if(recipients.some(object=>!/^(?:it|this (?:creature|permanent)|that (?:creature|permanent)|target (?:creature|permanent)(?: you control)?|a (?:creature|permanent) you control|each (?:attacking )?creature you control)$/i.test(object.trim())&&object.trim().toLowerCase()!==sourceName.toLowerCase()))return original;
   // A result gate has one precise counter-choice antecedent, never an
   // ambiguous group/indefinite recipient choice or a different actor.
   if(unavailableFollowup&&(recipients.length!==1||/^(?:each |a )/i.test(recipients[0])||actor==='that player'))return original;
   if(unavailableFollowup&&!/^Choose a kind of counter that \{Object\} (?:doesn't|doesnt|does not) have on it from among \{CounterKinds\}\. Put a counter of that kind on \{Object\}\.?$/i.test(def.reminder.replace(/\s+/g,' ').trim()))return original;
   const kinds=bullets.trim().split('\n').map(line=>line.replace(/^[ \t]*[•*-][ \t]+/,'').trim());
   if(!kinds.length||kinds.some(kind=>!/^([+-]\d+\/[+-]\d+|[A-Za-z][A-Za-z' -]{0,63}(?:\s+(?:\{[^{}\s]+\})+)?)$/.test(kind)))return original;
   const distinct=[...new Set(kinds.map(kind=>kind.toLowerCase()))];if(distinct.length!==kinds.length)return original;
   const menu=distinct.length===1?distinct[0]:distinct.slice(0,-1).join(', ')+' and '+distinct.at(-1);
   return recipients.map(object=>{
    const group=object.trim().match(/^each (.+)$/i);
    const indefinite=/^a (creature|permanent) you control$/i.exec(object.trim());
    const recipient=group?'that creature':indefinite?`that ${indefinite[1]}`:object.trim();
    let program=namedActionProgramDefinition(def).expansion.replace(/\{counterKinds\}/gi,menu).replace(/\{object\}/gi,recipient);
    if(actor) {
     if(def.actionProgram===true && def.collectiveObject===true) program=program.replace(/^Perform\b/i,actor==='you'?'You perform':'That player performs');
     else program=program.replace(/^choose\b/i,actor==='you'?'You choose':'That player chooses');
    }
    const selection=indefinite?`${actor==='that player'?'That player chooses':'You choose'} ${object.trim()}. `:'';
    return selection+(group?`For each ${group[1]}, ${program}`:program);
   }).join(' ')+(unavailableFollowup?' When no choice was available this way,'+unavailableFollowup.replace(/^When you (?:can't|cannot),/i,''):'')+'\n';
  });
 }
 return result;
}
