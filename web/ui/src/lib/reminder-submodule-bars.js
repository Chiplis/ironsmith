import {readLabeledAbilitySections} from './reminder-work-bars.js';
import {mapExecutableReferenceText} from './reminder-declared-object-references.js';
// Section metadata selects actual activated occurrences; it never resolves
// copied effect instructions or bypasses ordinary activation target choices.
function submoduleBinding(definition) {
 const sentences=String(definition.reminder).split('.').map(value=>value.trim().toLowerCase()).filter(Boolean);
 if(!sentences.includes('flavor-word bars name submodules'))return null;
 const required=['flavor-word bars name submodules','this layout has three submodule bars','each bar is an activated ability with its printed cost and complete section effect','activate each bar only once','activate bars only as a sorcery or when instructed to activate a submodule','when this permanent enters the battlefield you may activate one of its submodules without paying its activation cost'];
 if(sentences.length!==required.length||required.some(sentence=>!sentences.includes(sentence)))throw Error('A submodule layout must preserve every source cost, section, use cap, timing exception and optional entry activation.');
 const designation=String(definition.keyword).normalize('NFKC').trim().toLowerCase();
 if(!/^[a-z][a-z -]{0,63}$/.test(designation))throw Error('A submodule declaration needs one plain ability designation.');
 return {designation,action:designation};
}
// A consumer instruction is expanded only from the source's complete layout
// declaration. Quoted names and printed-text operands remain literal.
export function expandSubmoduleActivationReferences(text,definitions) {
 const pattern=/\bactivate (?:a|one) submodule of ([^.;\n]+?) without paying its activation cost\b/gi;
 if(!pattern.test(String(text)))return text;
 const bindings=definitions.map(submoduleBinding).filter(Boolean);
 if(!bindings.length)return text;
 const identities=new Map(bindings.map(binding=>[binding.designation,binding]));
 if(identities.size!==1)throw Error('A submodule reference needs one unambiguous declared ability family.');
 const binding=[...identities.values()][0];
 return mapExecutableReferenceText(text,part=>part.replace(pattern,(_whole,source)=>
  `activate an activated ability with designation ${binding.designation} of ${source} without paying its costs during ${binding.action} actions`));
}
export function expandSubmoduleBarProgram(text,definition) {
 const binding=submoduleBinding(definition);if(!binding)return null;
 const {prefix,sections}=readLabeledAbilitySections(text,3);let family=2166136261;
 for(const code of String(definition.keyword).normalize('NFKC').toLowerCase())family=Math.imul(family^code.codePointAt(0),16777619)>>>0;
 const result=[...prefix,`When this permanent enters the battlefield, you may activate an activated ability with designation ${binding.designation} of this permanent without paying its costs during ${binding.action} actions.`];
 for(let index=0;index<sections.length;index++){
  const bar=sections[index];const body=bar.lines.filter(line=>line.trim()).join(' ').trim();if(!body)throw Error('Every submodule requires its complete printed effect.');
  result.push(`Family ${family} Once group 0 step ${index} — ${bar.cost}: ${body.replace(/[.!?]*$/,'.')} This activated ability has designation ${binding.designation}. Activate only as a sorcery or during ${binding.action} actions.`);
 }
 return result.join('\n');
}
