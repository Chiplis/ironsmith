import {namedActionProgramDefinition} from './reminder-named-action-programs.js';
import {actionParameterPattern,expandActionParameterDeclarations} from './reminder-action-parameters.js';
import {mapExecutableReferenceText} from './reminder-declared-object-references.js';
// Explicit event forms bind source syntax to immutable operation receipts.
// Only complete self-action trigger paragraphs own these body references.
import {customKeywordForms} from './keyword-forms.js';
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
// Receipt predicates and their producer invocations must both be outside
// literal quotation, just as ordinary named-action reference expansion is.
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
const nouns = 'creature|permanent|artifact|enchantment|land|planeswalker|battle|card|object';
function bindActionBody(body, forms, pluralObjects) {
  const past = escape(forms.participle);
  return body.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index) => {
    if (index % 2) return part;
    // Quantities read the event's completed object count; collection targets
    // read its retained objects rather than a new filter over the battlefield.
    part = part.replace(new RegExp(`\\bthe number of (?:${nouns})s it ${past}\\b`, 'gi'), 'that many');
    part = part.replace(new RegExp(`\\beach (${nouns}) it ${past}\\b`, 'gi'), (_,noun) => `each of those ${noun.toLowerCase()}s`);
    part = part.replace(new RegExp(`\\b(${nouns})s it ${past}\\b`, 'gi'), (_,noun) => `those ${noun.toLowerCase()}s`);
    // With plural action objects, singular pronouns still name the performer.
    // A singular Vision recipient keeps the ordinary card antecedent.
    if (pluralObjects) part = part.replace(/\bits\b/gi, "this permanent's").replace(/\bit\b/gi, 'this permanent');
    return part;
  }).join('');
}
// A zero-operand player event has a performer but no affected-object packet.
// Past references query that player's completed history; they never confer a
// permanent designation or depend on the presence of a producer permanent.
function bindPlayerActionReferences(text, forms, label) {
  const verb=escape(forms.eventVerb),past=escape(forms.participle);
  return mapExecutableReferenceText(text,part=>part
    .replace(new RegExp(String.raw`(^|\n)(\s*)(Whenever|When)\s+(you|a player|an opponent)\s+${verb}s?(?=\s*,)`,'gi'),(_,line,space,intro,actor)=>`${line}${space}${intro} ${actor} ${/^you$/i.test(actor)?'perform':'performs'} the ${label} action`)
    .replace(new RegExp(String.raw`\byou\s+(?:have not|haven't|never)\s+${past}\b`,'gi'),`you have not performed the ${label} action`)
    .replace(new RegExp(String.raw`\byou\s+(?:have\s+)?${past}\b`,'gi'),(match,offset,whole)=>{
      const after=whole.slice(offset+match.length);
      // A local branch's object pronoun is not unqualified player history.
      // Its complete-program carrier owns the rewrite above; absent that
      // proof the original remains unsupported instead of gaining history.
      if(/\b(?:if|unless)\s*$/i.test(whole.slice(0,offset))&&/^\s+(?:it|that (?:creature|permanent|artifact|enchantment|land|card|object))\b/i.test(after))return match;
      return `you have performed the ${label} action`;
    }));
}
// A simple-past branch reference belongs to a declared complete preceding
// program that explicitly contains this named marker. It never becomes a
// turn-history predicate or a fresh invocation of the action.
function bindPriorProgramActionReceipts(text,definitions,faceName){
 let result=String(text);
 for(const marker of definitions){
  const forms=customKeywordForms(marker);if(!forms.eventVerb)continue;
  const label=forms.eventLabel||marker.keyword;
  const markerAtom=new RegExp(`\\bperform the ${escape(label)} action\\b`,'i');
  const carriers=definitions.filter(def=>def.actionProgram===true&&markerAtom.test(String(def.reminder||'')));
  if(!carriers.length)continue;
  const verbs=[forms.eventVerb,forms.participle].filter(Boolean).map(escape).join('|');
  const condition=new RegExp(`\\b(if) (?:you|they|that player) (?:have |has )?(?:${verbs}) (?:it|that (?:creature|permanent|artifact|enchantment|land|card|object))\\b(?![ \\t]+(?:this (?:turn|combat)|last turn|during|before)\\b)`,'gi');
  result=result.split('\n').map(line=>line.replace(condition,(whole,intro,offset)=>{
   if(!outsideQuotation(line,offset))return whole;
   const prefix=line.slice(0,offset);let preceding=null;
   for(const carrier of carriers){
    for(const invocation of prefix.matchAll(actionParameterPattern(carrier))){
     if(!outsideQuotation(prefix,invocation.index))continue;
     if(expandActionParameterDeclarations(invocation[0],namedActionProgramDefinition(carrier),faceName)===invocation[0])continue;
     if(!preceding||invocation.index>preceding.index)preceding={index:invocation.index,end:invocation.index+invocation[0].length,carrier};
    }
   }
   if(!preceding)return whole;
   const between=prefix.slice(preceding.end).trim().replace(/^[.;]\s*/,'').replace(/^then\s*/i,'');
   // Consecutive receipt-classifying If rows share the same producer. A
   // separate unconditional instruction ends this lexical receipt scope.
   if(between){
    const allowed=definitions.flatMap(def=>{
     const vocabulary=customKeywordForms(def);if(!vocabulary.eventVerb)return [];
     const atom=new RegExp(`\\bperform the ${escape(vocabulary.eventLabel||def.keyword)} action\\b`,'i');
     return atom.test(String(preceding.carrier.reminder||''))?[vocabulary.eventVerb,vocabulary.participle].filter(Boolean):[];
    }).map(escape).join('|');
    const raw=new RegExp(`^if (?:you|they|that player) (?:have |has )?(?:${allowed}) (?:it|that (?:creature|permanent|artifact|enchantment|land|card|object))\\b`,'i');
    let quoted=false,part='',rows=[];
    for(const char of between){
     if(/["“”]/.test(char))quoted=!quoted;
     if(!quoted&&/[.;]/.test(char)){rows.push(part);part='';}else part+=char;
    }
    rows.push(part);
    if(rows.map(row=>row.trim().replace(/^then\s+/i,'')).filter(Boolean)
      .some(row=>!/^if the preceding program included the\b/i.test(row)&&!raw.test(row)))return whole;
   }
   return `${intro} the preceding program included the ${label} action`;
  })).join('\n');
 }
 return result;
}

export function expandNamedActionReferences(text, definitions, faceName = '') {
  let result = bindPriorProgramActionReceipts(text,definitions,faceName);
  const meanings=new Map();
  for(const definition of definitions){
    const forms=customKeywordForms(definition);if(!forms.eventVerb)continue;
    const label=forms.eventLabel||definition.keyword;
    for(const form of [forms.eventVerb,forms.participle]){
      const previous=meanings.get(form);
      if(previous&&previous!==label)throw Error(`Ambiguous named action reference "${form}" names both "${previous}" and "${label}".`);
      meanings.set(form,label);
    }
  }
  for (const definition of definitions) {
    const forms = customKeywordForms(definition), verb = forms.eventVerb;
    if (forms.eventPaymentNoun) {
      const subjects = ['this permanent', 'this creature', 'this artifact', ...(faceName ? [faceName] : [])].map(escape).join('|');
      const passive = new RegExp(`^(\\s*)(Whenever|When)\\s+(?:${subjects})(?:'s|’s)\\s+${escape(forms.eventPaymentNoun)}\\s+is\\s+paid,([^\\n]*)$`, 'gim');
      result = result.replace(passive, (_,space,intro,body) =>
        `${space}${intro} this permanent performs the ${forms.eventLabel || definition.keyword} action,${body}`);
    }
    if (!verb) continue;
    // A declared noun phrase may name one completion, with an optional
    // recipient quality introduced by explicitly authored vocabulary.
    if (forms.eventObjectPreposition) {
      const prep = escape(forms.eventObjectPreposition);
      const completion = new RegExp(`^(\\s*)(Whenever|When)\\s+(you|a player|an opponent)\\s+${escape(verb)}(?:\\s+${prep}\\s+([^,;\\n]+))?,([^\\n]*)$`, 'gim');
      result = result.replace(completion, (_, space, intro, actor, objects, body) =>
        `${space}${intro} ${actor} ${actor.toLowerCase() === 'you' ? 'perform' : 'performs'} the ${forms.eventLabel || definition.keyword} action${objects ? ` with ${objects}` : ''},${body}`);
    }
    result = bindPlayerActionReferences(result, forms, forms.eventLabel || definition.keyword);
    const subjects = ['this permanent', 'this creature', 'this artifact', ...(faceName ? [faceName] : [])].map(escape).join('|');
    // A passive recipient predicate names the exact operation performer.
    // Explicit event metadata authorizes this relation; a status participle
    // without an event definition never takes this path.
    if (verb) {
      const relation=new RegExp(`\\b(it|that creature|that permanent)\\s+was\\s+${escape(forms.participle)}\\s+by\\s+(?:${subjects})\\s+this\\s+turn\\b`,'gi');
      result=result.split('\n').map(line=>/^\s*(?:When|Whenever)\b/i.test(line)
        ? line.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index)=>index%2?part:part.replace(relation,
          ()=>`it was affected by the ${forms.eventLabel || definition.keyword} action performed by this permanent this turn`)).join('')
        :line).join('\n');
    }

    const pattern = new RegExp(`^(\\s*)(Whenever|When)\\s+(?:${subjects})\\s+${escape(verb)}s?\\s+([^,;\\n]+),([\\s\\S]*)$`, 'i');
    result = result.split('\n').map(line => {
      // A shared singular operand in a coordinated player event denotes two
      // alternative occurrences, never two effects or a new action producer.
      const coordinated = new RegExp(`^(\\s*)(Whenever|When)\\s+(you|a player|an opponent)\\s+draws? or ${escape(verb)}s? (a card),([\\s\\S]*)$`, 'i').exec(line);
      if (coordinated) {
        const [,space,intro,actor,object,body]=coordinated;
        const person=actor.toLowerCase();
        return `${space}${intro} ${person} ${person==='you'?'draw':'draws'} ${object} or ${person==='you'?'perform':'performs'} the ${forms.eventLabel || definition.keyword} action with ${object},${body}`;
      }
      const cost = new RegExp(`((?:this spell|it) costs? [^.!?\\n]*?less to cast for each (${nouns})s?) ${escape(forms.participle)} this way`, 'gi');
      line = line.replace(cost, '$1 affected this way');
      const match = pattern.exec(line);
      if (!match) {
        // Player-action one-or-more clauses consume one complete packet. A
        // singular per-recipient trigger is deliberately a different reader.
        const player = new RegExp(`^(\\s*)(Whenever|When)\\s+(you|a player|an opponent)\\s+${escape(verb)}s?\\s+one or more\\s+((?:${nouns})s)(?:\\s+([^,;\\n]+))?,([\\s\\S]*)$`, 'i').exec(line);
        if (!player) {
          const singular = new RegExp(`^(\\s*)(Whenever|When)\\s+(you|a player|an opponent)\\s+${escape(verb)}s?\\s+(a|an|one)\\s+(${nouns})(?:\\s+([^,;\\n]+))?,([\\s\\S]*)$`, 'i').exec(line);
          if (!singular) return line;
          const [,space,intro,actor,article,noun,qualifier,body]=singular;
          const person=actor.toLowerCase();
          return `${space}${intro} ${person} ${person==='you'?'perform':'performs'} the ${forms.eventLabel || definition.keyword} action with ${article} ${noun}${qualifier?` ${qualifier}`:''},${body}`;
        }
        const [,space,intro,actor,noun,qualifier,body] = player;
        const person = actor.toLowerCase();
        const operation = person === 'you' ? 'perform' : 'performs';
        const objects = noun + (qualifier ? ` ${qualifier}` : '');
        return `${space}${intro} ${person} ${operation} the ${forms.eventLabel || definition.keyword} action with ${objects},${body}`;
      }
      const [,space,intro,objects,body] = match;
      const pluralObjects = new RegExp(`\\b(?:${nouns})s\\b`, 'i').test(objects);
      return `${space}${intro} this permanent performs the ${forms.eventLabel || definition.keyword} action with ${objects},${bindActionBody(body, forms, pluralObjects)}`;
    }).join('\n');
  }
  return result;
}
