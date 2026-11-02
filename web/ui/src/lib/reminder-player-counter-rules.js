// Inline player-counter meanings are declarations, not permanent ability counters.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function expandPlayerCounterRuleDeclarations(source) {
 return String(source).replace(/((?:Target player|That player|You)\s+(?:gets?|get)\s+(?:one|two|three|four|five|six|seven|eight|nine|ten|\d+)\s+([a-z][a-z -]*?)\s+counters?\.)\s*\*?\(([^()]+)\)\*?/gi,(original,placement,name,reminder)=>{
  const phase=reminder.match(/^At your (first main phase),\s*(.+)$/i);if(!phase)return original;
  const loop=phase[2].match(new RegExp(`^for each ${escape(name)} counter on you,\\s*(.+)$`,'i'));if(!loop)return original;
  // Resolve the loop-bound counter noun and its elided removal verb; the
  // rest remains an ordinary effect program for the native readers.
  let body=loop[1].replace(/\band the counter\b/gi,`and remove a ${name} counter from yourself`);
  // A singular card predicate immediately after an ordinary mill instruction
  // names that instruction's receipt, including a zero-card result.
  const authored=body;
  body=body.replace(/\bIf (?:it's|it is) nonland\b/gi,(match,offset)=>
   /(?:^|[.;]\s*)mill a card\.\s*$/i.test(authored.slice(0,offset))
    ? 'If a nonland card was milled this way' : match);

  body=body.replace(/[.!?]+$/,'');
  const owner=/^You\b/i.test(placement)?'you':'that player';
  const key=`${name.toLowerCase()} player counter meaning`;
  const condition=`You have one or more ${name} counters and it's the first main phase of the turn`;
  const ability=`At the beginning of each of your main phases, For each ${name} counter on you, do the following: ${body}`;
  return `${placement} Register a persistent rule named "${key}" for ${owner} with event condition "${condition}" and ability "${ability}".`;
 });
}
