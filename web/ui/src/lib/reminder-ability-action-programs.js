// Compositional shorthand for an explicitly declared Object/Ability action.
// Native bounded programs own branch scope and resolution-time choice.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const abilities=['first strike','double strike','deathtouch','defender','flying','haste','hexproof','indestructible','lifelink','menace','reach','trample','vigilance'];
const ability='(?:'+abilities.join('|')+')';
const list=text=>text.split(/,|\s+or\s+|\s+and\s+/i).map(s=>s.trim().replace(/^(?:or|and)\s+/i,'').toLowerCase()).filter(Boolean);
function bounded(text){let quote=false,depth=0;return '['+[...text.trim()].map(c=>{if(/["“”]/.test(c)){quote=!quote;return c}if(quote)return c;if(c==='[')depth++;if(c===']')depth--;return !depth&&/[.!?]/.test(c)?';':c}).join('').replace(/[;\s]+$/,'')+']';}
export function expandAbilityActionPrograms(text,definition,render){
 if(!/^\{object\}\s+\{ability\}$/i.test(String(definition.template||'')))return String(text);
 const keyword=escape(definition.keyword);
 return String(text).split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index)=>{
  if(index%2)return part;
  const conditional=new RegExp(`(^|[,.;:]\\s*)${keyword} ([^.!?;:\\n]+?) (${ability}) if ([^.!?;:\\n]+)[.](?:\\s+The same is true for ([^.!?;:\\n]+)[.])?(?:\\s+Otherwise, ([^.!?;:\\n]+)[.])?`,'gim');
  part=part.replace(conditional,(whole,prefix,object,first,condition,others,otherwise)=>{
   const choices=others?list(others):[];
   if(choices.some(a=>!abilities.includes(a))||choices.length>13||others&&!new RegExp(`\\b${escape(first)}\\b`,'i').test(condition)||others&&otherwise)return whole;
   const programs=[];
   for(const selected of [first,...choices]){const invocation=`${definition.keyword} ${object} ${selected}`;const body=render(invocation);if(body===invocation)return whole;const predicate=selected===first?condition:condition.replace(new RegExp(`\\b${escape(first)}\\b`,'gi'),selected);programs.push(`If ${predicate}, do the following: ${bounded(body)}`);}
   return prefix+programs.join('. ')+'.'+(otherwise?` Otherwise, do the following: ${bounded(otherwise)}.`:'');
  });
  const choice=new RegExp(`(^|[,.;:]\\s*)${keyword} ([^.!?;:\\n]+?) your choice of ([^.!?;:\\n]+?)(?=[.\\n]|$)(?:[.]\\s+Then repeat this process (x|one|two|three|four|five|six|seven|eight|nine|ten|\\d+) times[.])?`,'gim');
  return part.replace(choice,(whole,prefix,object,options,repeat)=>{
   const choices=list(options);if(choices.length<2||choices.length>14||choices.some(a=>!abilities.includes(a))||/\btarget\b/i.test(object))return whole;
   const programs=choices.map(selected=>{const invocation=`${definition.keyword} ${object} ${selected}`;const result=render(invocation);return result===invocation?null:bounded(result)});
   if(programs.some(p=>!p))return whole;
   const program='Choose one of the following: '+programs.join(' or ');
   // Make the previous process explicit before the bounded modal reader
   // dispatches its following sentences in a separate document context.
   return repeat?prefix+'Do the following: ['+program+'; Repeat the following process '+repeat+' times: ['+program+']].':prefix+program;
  });
 }).join('');
}
