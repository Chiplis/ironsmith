// A relationship label comes only from a declared ordinary link operation.
// Its source participle is vocabulary; no card/mechanic name selects behavior.
import {customKeywordForms} from './keyword-forms.js';
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
const outside=(text,offset)=>(text.slice(0,offset).match(/["“”]/g)||[]).length%2===0;
function map(text,pattern,replace){const before=String(text);return before.replace(pattern,(...args)=>{
 const offset=args.at(-2);return outside(before,offset)?replace(...args):args[0];
});}
export function exileRelationship(definition){
 if(definition.actionProgram!==true||definition.collectiveObject!==true)return null;
 const match=String(definition.reminder).match(/\blink the iterated (?:card|object) to the chosen permanent under the ([a-z][a-z '-]{0,63}) relationship\b/i);
 return match?{name:match[1].toLowerCase(),participle:customKeywordForms(definition).participle}:null;
}
export function expandExileRelationshipReferences(text,definition,sourceName=''){
 const relation=exileRelationship(definition);if(!relation)return String(text);
 const past=escape(relation.participle),name=relation.name;
 const host=[sourceName&&escape(sourceName),sourceName.includes(',')&&escape(sourceName.split(',')[0].trim()),'this (?:permanent|creature|artifact|enchantment|planeswalker|land|source)','permanents you control','it','them'].filter(Boolean).join('|');
 const canonicalHost=value=>/^(?:it|them)$/i.test(value)?'this permanent':value;
 let result=String(text).split('\n').map(line=>{
  const pattern=new RegExp(`^Whenever (.+?cards?(?: with [^,]+)?) (?:is|are) ${past} onto (${host}),[ \\t]*(.*)$`,'i');
  const match=line.match(pattern);if(!match)return line;
  let body=match[3].replace(/\byou may have it become\b/i,'you may have the action performer become');
  return `Whenever ${canonicalHost(match[2])} performs the ${name} action with ${match[1]}, ${body}`;
 }).join('\n');
 // An implicit host-relative collection stays on each candidate permanent.
 result=map(result,new RegExp(`\\bwith[ \\t]+((?:(?:one|two|three|four|five|six|seven|eight|nine|ten|\\d+) or more[ \\t]+)?cards?)[ \\t]+${past} onto (?:it|them)\\b`,'gi'),(_all,members)=>`with ${members} linked under the ${name} relationship`);
 result=map(result,new RegExp(`\\bas long as[ \\t]+([^,.\\n]+?)[ \\t]+(is|are)[ \\t]+${past} onto (${host})\\b`,'gi'),(_all,members,copula,anchor)=>`as long as there ${copula} ${members} linked to ${canonicalHost(anchor)} under the ${name} relationship`);
 const card='(?:(?:artifact|creature|enchantment|land|instant|sorcery|permanent|planeswalker|battle|nonland|nontoken|legendary|or|and)[ \\t]+)*cards?(?:[ \\t]+with[ \\t]+[^,.\\n]+?)?';
 result=map(result,new RegExp(`\\b(${card})[ \\t]+${past} onto (${host})\\b`,'gi'),(_all,members,anchor)=>`${members} linked to ${canonicalHost(anchor)} under the ${name} relationship`);
 return result;
}
