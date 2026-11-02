// Explicit card-creation declarations use caller-supplied real-card identities.
// Definitions are data; names never select a token or the official catalog.
import {validateStandaloneSuppliedDefinitionMetadata} from './supplied-definition-admission.js';
import {expandCreatedCardReferences} from './reminder-card-creation-references.js';
import {expandExtendedCardCreationOperands} from './reminder-card-creation-operands.js';
import {customKeywordForms} from './keyword-forms.js';
import {expandCreationKeywordTriggerBridges} from './reminder-creation-keyword-triggers.js';
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const canonical=value=>JSON.stringify(value,(name,entry)=>entry&&typeof entry==='object'&&!Array.isArray(entry)?Object.fromEntries(Object.entries(entry).sort(([a],[b])=>a.localeCompare(b))):entry);
const key=value=>String(value).normalize('NFKC').replace(/\s+/g,' ').trim().toLowerCase();
const amount='(?:a|an|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|\\d+|X|that many)';
const quote=value=>`"${String(value)}"`;
export function suppliedCardDefinitions(draft){
 const definitions=[...(draft.suppliedCardDefinitions||[]),...(draft.faces||[]).flatMap(face=>face.suppliedCardDefinitions||[])];
 const names=new Map();
 for(const definition of definitions){
  if(!definition||typeof definition.name!=='string'||!definition.name.trim()||typeof definition.text!=='string'||!definition.text.trim())throw new Error('A supplied real-card definition needs its complete name and native text.');
  if(/["“”\r\n]/.test(definition.name))throw new Error('A supplied definition name cannot contain a quote or line break.');
  if(definition.token===true)throw new Error('Supplied creation definitions must describe real cards.');
  validateStandaloneSuppliedDefinitionMetadata(definition);
  const normalized={...structuredClone(definition),name:definition.name.normalize('NFKC').trim(),text:definition.text.trim()};
  const identity=key(normalized.name),prior=names.get(identity);
  if(prior&&canonical(prior)!==canonical(normalized))throw new Error(`Conflicting supplied real-card definition "${normalized.name}".`);
  names.set(identity,normalized);
 }
 return [...names.values()];
}
export function expandCardCreationPrograms(text,definitions,pool=[],sourceName=''){
 let result=String(text);
 for(const definition of definitions){
  if(definition.declarationKind!=='card-creation')continue;
  const forms=customKeywordForms(definition),label=key(definition.keyword),verb=escape(forms.name),past=escape(forms.participle);
  const names=pool.map(def=>def.name).sort((a,b)=>b.length-a.length).map(escape).join('|');
  const apply=body=>{
   let value=expandCreationKeywordTriggerBridges(body,definition);
   value=expandExtendedCardCreationOperands(value,{label,verb,names,past});
   // Observers consume exact ordinary named-operation receipts, not a new
   // card-creation keyword trigger or a source-wide synthetic counter.
   value=value.replace(new RegExp(`\\b(When|Whenever)\\s+(you|a player|an opponent)\\s+${verb}(?:s)?\\s+(one or more|a|an)\\s+((?:(?:[a-z]+|[+-]?[0-9]+/[0-9]+)\\s+)*?cards?)(?:\\s+(into your hand|into their hand|onto the battlefield|into your graveyard|into their graveyard))?(?=\\s*,)`,'gi'),(_,when,actor,quantity,quality,destination)=>{
    const plural=key(quantity)==='one or more';
    const zone=!destination?'':/battlefield/i.test(destination)?' on the battlefield':/hand/i.test(destination)?(key(actor)==='you'?' in your hand':' in a hand'):(key(actor)==='you'?' in your graveyard':' in a graveyard');
    return `${when} ${plural?'one or more':quantity} ${quality}${zone} ${plural?'participate':'participates'} in the ${label} action performed by ${actor}`;
   });
   value=value.replace(new RegExp(`\\bSpells? you cast with the same name as a card you['’]ve ${past} this turn cost`,'gi'),()=>`Spells you cast sharing a name with a card from your ${label} action this turn cost`);
   // Random-batch selection and repeated independent operations are
   // separate compositional shapes. A repeated operation completes once per
   // iteration, rather than pretending one bulk receipt was N operations.
   const randomDestination=`(?:into your hand|onto the battlefield|and put it on top of your library|and shuffle it into your library)`;
   const random=new RegExp(`\\b${verb}\\s+(?:(${amount})\\s+of\\s+)?a random\\s+([^.;\\n]+?)\\s+card\\s+(${randomDestination})(?:\\s+(${amount})\\s+times)?`,'gi');
   value=value.replace(random,(original,batch,quality,destination,repetitions)=>{
    if(batch&&repetitions)return original;
    const target=/battlefield/i.test(destination)?'on the battlefield':/library/i.test(destination)?'on top of your library':'in your hand';
    let program=`Create ${batch||'a'} real cards matching randomly ${quality} card ${target} as part of the named ${label} action`;
    if(/shuffle/i.test(destination))program+=`; Shuffle your library`;
    return repetitions?`Repeat the following process ${repetitions} times: [${program}]`:program;
   });
   const duplicate=new RegExp(`\\b${verb}\\s+a duplicate of\\s+([^.;\\n]+?)\\s+(${randomDestination})`,'gi');
   value=value.replace(duplicate,(_,object,destination)=>{
    const target=/battlefield/i.test(destination)?'on the battlefield':/library/i.test(destination)?'on top of your library':'in your hand';
    const program=`Create a real card duplicating the ${object} ${target} as part of the named ${label} action`;
    return /shuffle/i.test(destination)?`${program}; Shuffle your library`:program;
   });
   // A creation proposal is defined by an operation label and an authored
   // supplied identity, not a ChooseSpec over already existing objects. Keep
   // this dedicated typed interface even before the caller supplies the
   // referenced definition: native supplied-definition admission still owns
   // resolving that exact identity and fails closed when it is absent.
   const proposedIdentity=new RegExp(`\\bIf a player would ${verb} a card, they instead ${verb} a card named (?:["“]([^"”\\n]+)["”]|([^.;\\n]+))\\.`,'gi');
   value=value.replace(proposedIdentity,(_,quotedName,printedName)=>{
    const authored=(quotedName||printedName).trim();
    const supplied=pool.find(record=>key(record.name)===key(authored));
    return `If a player would create real cards as part of the named ${label} action, use the supplied card definition named ${quote(supplied?.name||authored)} instead.`;
   });
   if(names){
    const item=`${amount}\\s+cards?\\s+named\\s+(?:${names})`;
    const items=`${item}(?:(?:,\\s*(?:(?:and|or)\\s+)?|\\s+(?:and|or)\\s+)${item})*`;
    const destination=`(?:into (?:your|their|each player['’]s) hand|onto the battlefield|and put (?:it|them) on top of your library|and shuffle (?:it|them) into your library)`;
    const declaration=new RegExp(`\\b(?:(you may have each player|each player|you may|they may)\\s+)?${verb}\\s+(${items})\\s+(${destination})(?:\\s+if\\s+([^.;\\n]+))?`,'gi');
    value=value.replace(declaration,(original,actor,selection,destination,condition)=>{
     const many=key(actor||'').includes('each player')||/each player['’]s/i.test(destination);
     const optional=/may/i.test(actor||'');
     const library=/library/i.test(destination),shuffle=/shuffle/i.test(destination);
     const target=/battlefield/i.test(destination)?'on the battlefield':library?'on top of your library':many?"in that player's hand":'in your hand';
     const itemReader=new RegExp(`(${amount})\\s+cards?\\s+named\\s+(${names})`,'gi');
     const matches=[...selection.matchAll(itemReader)];
     const joins=matches.slice(1).map((match,index)=>selection.slice(matches[index].index+matches[index][0].length,match.index));
     const conjunctions=joins.flatMap(join=>[...join.matchAll(/\b(and|or)\b/gi)].map(match=>match[1].toLowerCase()));
     if(conjunctions.includes('and')&&conjunctions.includes('or'))return original;
     const selections=matches.map(match=>({count:/^an?$/i.test(match[1])?'a':match[1],name:match[2]}));
     if(!selections.length)return original;
     const native=({count,name})=>`Create ${count} real cards named ${quote(name)} ${target} as part of the named ${label} action`;
     let program;
     if(selections.length===1)program=native(selections[0]);
     else if(conjunctions.includes('or'))program=`Choose one of the following: ${selections.map(value=>`[${native(value)}]`).join(' or ')}`;
     else if(selections.every(value=>/^(?:a|one)$/i.test(value.count)))program=`Create a real card for each name ${selections.map(value=>quote(value.name)).join(', ')} ${target} as part of the named ${label} action`;
     else return original;
     if(shuffle)program=`${program}; Shuffle your library`;
     if(many)program=`For each player, do the following: [${program}]`;
     if(optional)program=`${/^they/i.test(actor)?'That player':'You'} may do the following: [${program}]`;
     if(condition)program=`If ${condition.trim()}, ${program}`;
     return program;
    });
   }
   return value;
  };
  // Quoted card names are opaque. Only explicitly granted complete ability
  // bodies can contain an independently executable nested creation action.
  result=result.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index,parts)=>{
   if(!(index%2))return apply(part);
   const before=parts.slice(0,index).join('');const body=part.slice(1,-1);
   if(!/\b(?:gains?|has|have|with)\s*$/i.test(before)||! /^(?:Once each turn,\s*)?(?:When|Whenever|At)\b/i.test(body))return part;
   const expanded=apply(body);return expanded===body?part:part[0]+expanded.replace(/"/g,"'")+part.at(-1);
  }).join('');
  result=expandCreatedCardReferences(result,{label,verb,past,sourceName});
 }
 return result;
}
export const hasCardCreationProgram=(text,definition,pool=[],sourceName='')=>expandCardCreationPrograms(text,[definition],pool,sourceName)!==String(text);
