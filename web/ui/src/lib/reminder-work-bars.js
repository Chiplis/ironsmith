// Layout bars compose existing activated abilities, noncopiable status
// designations, and conditional ability grants. Names and costs are data.
export function readWorkBarProgram(reminder) {
 const sentences=String(reminder).split('.').map(value=>value.trim().toLowerCase()).filter(Boolean);
 if(!sentences.includes('flavor-word bars name works'))return null;
 const numbers={one:1,two:2,three:3,four:4,five:5,six:6,seven:7,eight:8,nine:9,ten:10};
 let count=null;
 const instructions=new Set(['flavor-word bars name works','each bar is an activated ability with its printed cost','resolving a bar unlocks its named work','activate a bar only while no work has been unlocked','activate bars only as a sorcery','an unlocked work grants the abilities printed in its section','unlocked works are noncopiable designations retained across type and control changes']);
 for(const sentence of sentences){
  const size=sentence.match(/^this layout has (\w+) work bars$/);
  if(size){if(count!==null)throw Error('A work layout must declare its bar count once.');count=numbers[size[1]]??Number(size[1]);continue;}
  if(!instructions.has(sentence))throw Error('Unrecognized work-bar instruction.');
 }
 if(!Number.isInteger(count)||count<1||count>32)throw Error('A work layout needs its exact bar count.');
 if([...instructions].some(instruction=>!sentences.includes(instruction)))throw Error('A work layout must preserve costs, resolution-time unlocking, eligibility, timing, grants and designation lifecycle.');
 return {count};
}
export function readLabeledAbilitySections(text, count) {
 const lines=String(text).split('\n');const prefix=[],works=[];let current=null;
 const costStart=/^(?:\{|pay\b|sacrifice\b|discard\b|remove\b|exile\b|tap\b|untap\b|return\b)/i;
 for(const line of lines){
  const head=line.match(/^\s*([^:]+?):\s*([^—–]+?)(?:\s*[—–]\s*(.*))?\s*$/u);
  const bar=head&&costStart.test(head[1])&&/^[\p{L}\p{N}][\p{L}\p{N}\s'’‘,:-]*$/u.test(head[2]);
  if(bar){current={cost:head[1].trim(),name:head[2].trim(),lines:head[3]?[head[3]]:[]};works.push(current);continue;}
  (current?current.lines:prefix).push(line);
 }
 if(works.length!==count)throw Error(`Layout requires ${count} complete labeled sections; found ${works.length}.`);
 return {prefix,sections:works};
}
export function expandWorkBarProgram(text,definition,expand=body=>({oracleText:body})) {
 const program=readWorkBarProgram(definition.reminder);if(!program)return null;
 const {prefix,sections:works}=readLabeledAbilitySections(text,program.count);
 const names=new Set();const result=[...prefix];
 for(const work of works){
  const key=work.name.toLowerCase();if(names.has(key))throw Error('Different work bars must retain distinct names.');names.add(key);
  const body=work.lines.filter(line=>line.trim()).join('\n');if(!body)throw Error('A work must retain its complete ability section.');
  const expanded=expand(body);
  const abilities=String(expanded.oracleText||'').split('\n').filter(line=>line.trim());
  if(!abilities.length)throw Error('A work must grant its complete ability program.');
  // Apostrophe quotes are the native representation for inner rules text;
  // the outer quote owns exactly one granted ability, including its sentences.
  const grants=[...(expanded.customKeywordIdentities||[]).map(name=>`custom keyword "${name}"`),
   ...abilities.map(ability=>{
    const program=ability.trim().replace(/["“”]/g,"'");
    // A nested keyword expansion may omit its final display period. The
    // quoted child still owns a complete ability; do not rely on the grant's
    // outer period or split a multi-sentence program at inner boundaries.
    return `"${/[.!?]$/.test(program)?program:program+'.'}"`;
   })];
  const status=`unlocked work named ${work.name}`;
  result.push(`${work.cost}: Give this permanent the "${status}" status. Give this permanent the unlocked work status. Activate only if this permanent does not have the unlocked work status. Activate only as a sorcery.`);
  result.push(`As long as this permanent has the "${status}" status, this permanent has ${grants.join(', and ')}.`);
 }
 return result.join('\n');
}
