// Bounded actor/operand syntax shared by explicitly declared creation actions.
const amount='(?:a|an|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|\\d+|X|that many)';
const key=value=>String(value||'').toLowerCase().replace(/\s+/g,' ').trim();
const quoted=value=>`"${value}"`;
const noun=count=>/^(?:a|an|one|1)$/i.test(count)?'card':'cards';
export function expandExtendedCardCreationOperands(text,{label,verb,names,past}) {
 const actor='(?:you may have each player|each player|target opponent|target player|its controller|its owner|that player|they|you)(?: may)?';
 const destinationCore="(?:into (?:your|their) (?:hand|graveyard|(?:main )?library)|onto the battlefield|on top of (?:your|their) library|and puts? (?:it|them) on top of (?:your|their) library|and shuffles? (?:it|them) into (?:your|their) (?:main )?library)";
 const destination=destinationCore+"(?:,?\\s+(?:then\\s+)?shuffle(?: (?:your|their) library)?)?";
 const operands=(actorText,dest)=>{
  const normalized=key(actorText),many=normalized.includes('each player');
  const optional=normalized.includes('may');
  const owner=many||/^(?:they|that player)/.test(normalized)?"that player's":/^target opponent/.test(normalized)?(optional?"that player's":"target opponent's"):/^target player/.test(normalized)?(optional?"that player's":"target player's"):/^its controller/.test(normalized)?"its controller's":/^its owner/.test(normalized)?"its owner's":/their/i.test(dest)?"that player's":"your";
  const zone=/battlefield/i.test(dest)?'on the battlefield':/library/i.test(dest)?`on top of ${owner} library`:/graveyard/i.test(dest)?`in ${owner} graveyard`:`in ${owner} hand`;
  const decider=/its owner/.test(normalized)?'Its owner':/its controller/.test(normalized)?'Its controller':/target opponent/.test(normalized)?'Target opponent':/target player/.test(normalized)?'Target player':/^(?:they|that player)/.test(normalized)?'That player':'You';
  return {many,optional,individualOptional:/^each player may$/.test(normalized),zone,decider,shuffle:(dest.match(/\bshuffles?\b/gi)||[]).length};
 };
 const wrap=(program,actorText,dest)=>{
  const role=operands(actorText,dest);
  for(let i=0;i<role.shuffle;i++)program+=`; Shuffle ${role.many?"that player's":/their/i.test(dest)?"that player's":"your"} library`;
  if(role.individualOptional)program=`That player may do the following: [${program}]`;
  if(role.many)program=`For each player, do the following: [${program}]`;
  if(role.optional&&!role.individualOptional)program=`${role.decider} may do the following: [${program}]`;
  return program;
 };
 let result=String(text);
 if(past){
  const history=new RegExp(`\\b(you['’]ve|you have|you haven['’]t|you have not)\\s+${past}\\s+(?:a|one or more)\\s+cards?\\s+this turn\\b`,'gi');
  result=result.replace(history,(whole,subject)=>`${subject[0]==='Y'?'You':'you'} have${/not|n['’]t/i.test(subject)?' not':''} performed the ${label} action with an amount of 1 or more this turn`);
 }

 // Reordered and counted duplicate operands use the same immutable-name selector.
 const duplicate=new RegExp(`\\b(?:(${actor})\\s+)?${verb}(?:s)?\\s+(?:(into (?:your|their) (?:hand|graveyard|(?:main )?library))\\s+)?(${amount})\\s+duplicates? of\\s+([^.;\\n]+?)(?:\\s+(${destination})|(?=[.;\\n]|$))`,'gi');
 result=result.replace(duplicate,(original,who,front,count,object,back)=>{
  const dest=back||front;if(!dest)return original;
  const role=operands(who,dest);
  return wrap(`Create ${/^an?$/i.test(count)?'a':count} real ${noun(count)} duplicating the ${object.trim()} ${role.zone} as part of the named ${label} action`,who,dest);
 });
 if(!names)return result;
 const first=`${amount}\\s+cards?\\s+named\\s+(?:${names})`;
 const next=`(?:(?:${amount})\\s+cards?\\s+named\\s+)?(?:${names})`;
 const selection=`${first}(?:(?:,\\s*(?:(?:and|or)\\s+)?|\\s+(?:and|or)\\s+)${next})*`;
 const named=new RegExp(`\\b(?:(${actor})\\s+)?${verb}(?:s)?\\s+(${selection})\\s+(${destination})(?:\\s+for each\\s+([^.;\\n]+?))?(?:\\s+if\\s+([^.;\\n]+))?(?=[.;\\n]|,\\s*then\\b|$)`,'gi');
 return result.replace(named,(original,who,selection,dest,perItem,condition)=>{
  const role=operands(who,dest);
  const reader=new RegExp(`(?:(${amount})\\s+cards?\\s+named\\s+)?(${names})`,'gi');
  const parts=[...selection.matchAll(reader)];if(!parts.length)return original;
  if(perItem&&parts.some(part=>!/^(?:a|an|one|1)$/i.test(part[1]||parts[0][1])))return original;
  const parsed=parts.map(part=>({count:perItem?`the number of ${perItem.trim().replace(/\b(card|permanent|creature|artifact|enchantment|land|spell|object|player)\b/i,'$1s')}`:part[1]||parts[0][1],name:part[2]}));
  const joins=parts.slice(1).map((part,index)=>selection.slice(parts[index].index+parts[index][0].length,part.index));
  const and=joins.some(join=>/\band\b/i.test(join)),or=joins.some(join=>/\bor\b/i.test(join));if(and&&or)return original;
  const create=item=>`Create ${/^an?$/i.test(item.count)?'a':item.count} real ${noun(item.count)} named ${quoted(item.name)} ${role.zone} as part of the named ${label} action`;
  let program;
  if(parsed.length===1)program=create(parsed[0]);
  else if(or)program=`Choose one of the following: ${parsed.map(item=>`[${create(item)}]`).join(' or ')}`;
  else if(parsed.every(item=>key(item.count)===key(parsed[0].count)))program=`Create ${/^an?$/i.test(parsed[0].count)?'a':parsed[0].count} real ${noun(parsed[0].count)} for each name ${parsed.map(item=>quoted(item.name)).join(', ')} ${role.zone} as part of the named ${label} action`;
  else return original;
  program=wrap(program,who,dest);
  return condition?`If ${condition.trim()}, ${program}`:program;
 });
}
