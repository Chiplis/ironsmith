// A program declaration is explicit metadata, separate from a completed marker.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
const replacementPattern = definition => new RegExp(`\\b(If[ \\t]+(?:you|an opponent|a player)[ \\t]+would)[ \\t]+${escape(definition.keyword)}(?=[ \\t]*(?:while[ \\t]+[^,.;\\n]+)?[ \\t]*,)`, 'gi');

export function expandNamedActionReplacementReferences(text, definitions) {
  let result = String(text);
  const planned = /\bIf[ \t]+(?:you|an opponent|a player)[ \t]+would[ \t]+perform[ \t]+the[ \t]+([a-z][a-z '-]{0,63})[ \t]+action(?:[ \t]+with[ \t]+one[ \t]+or[ \t]+more[ \t]+[^,.;\n]+)?(?=[ \t]*,)/gi;
  for (const match of result.matchAll(planned)) {
    if (!outsideQuotation(result, match.index)) continue;
    const definition = definitions.find(definition => definition.keyword.toLowerCase() === match[1].trim().toLowerCase());
    if (definition?.actionProgram !== true) throw new Error(`"${match[1]}" needs an actionProgram declaration before its proposed operation can be replaced.`);
    if (/\baction[ \t]+with[ \t]+one[ \t]+or[ \t]+more\b/i.test(match[0]) && definition.collectiveObject !== true) throw new Error('A proposed object set needs an explicit collective object action program.');
  }
  for (const definition of definitions) {
    // The explicit card-creation declaration has a separate proposal owner:
    // supplied identities and destinations are not existing-object operands.
    // Its source clauses are expanded by expandCardCreationPrograms first.
    if (definition.declarationKind === 'card-creation') continue;
    // A singular filtered operand is one proposed object, not the action's
    // performer. The native filter reader owns the ordinary qualifier.
    const singular = new RegExp(`\\b(If[ \\t]+(?:you|an opponent|a player)[ \\t]+would)[ \\t]+${escape(definition.keyword)}[ \\t]+(?:a|an)[ \\t]+((?:(?:non[a-z]+|[a-z]+)[ \\t]+)*(?:creature|permanent|artifact|enchantment|card)(?:[ \\t]+you[ \\t]+control)?)(?=[ \\t]*,)`, 'gi');
    const singularBefore = result;
    result = result.replace(singular, (match, prefix, operand, offset) => {
      if (!outsideQuotation(singularBefore, offset)) return match;
      if (definition.actionProgram !== true || definition.collectiveObject !== true) throw new Error('A proposed object needs an explicit collective object action program.');
      const plural = operand.replace(/\b(creature|permanent|artifact|enchantment|card)(?=(?:[ \t]+you[ \t]+control)?$)/i, '$1s');
      return `${prefix} perform the ${definition.keyword} action with one or more ${plural}`;
    });
    const grouped=new RegExp(`\\b(If[ \\t]+(?:you|an opponent|a player)[ \\t]+would)[ \\t]+${escape(definition.keyword)}[ \\t]+(one[ \\t]+or[ \\t]+more[ \\t]+cards)(?=[ \\t]*,)`,'gi');
    const groupedBefore=result;
    result=result.replace(grouped,(match,prefix,objects,offset)=>{
      if(!outsideQuotation(groupedBefore,offset))return match;
      if(definition.actionProgram!==true||definition.collectiveObject!==true)throw new Error('A proposed object set needs an explicit collective object action program.');
      return `${prefix} perform the ${definition.keyword} action with ${objects}`;
    });
    const before = result;
    result = result.replace(replacementPattern(definition), (match, prefix, offset) => {
      if (!outsideQuotation(before, offset)) return match;
      if (definition.actionProgram !== true) throw new Error(`"${definition.keyword}" needs an actionProgram declaration before its proposed operation can be replaced.`);
      return `${prefix} perform the ${definition.keyword} action`;
    });
  }
  return result;
}

export function namedActionProgramDefinition(definition) {
  if (definition.actionProgram !== true) return definition;
  if (/^Perform the .+ action(?:(?: on \{Object\}| with tokens created this way)? as follows| with tokens created this way if any are created)(?:, completing the action only if all choices were available)?:/i.test(definition.expansion)) {
    if (definition.completionRequirement === 'all-choices-available' && !/, completing the action only if all choices were available:/i.test(definition.expansion)) throw new Error('An already bounded action program must retain its explicit completion requirement.');
    return definition;
  }
  const body = String(definition.expansion || definition.reminder).trim().replace(/[.!?]+$/, '');
  let quoted = false;
  const bounded = [...body].map(char => {
    if (/["“”]/.test(char)) quoted = !quoted;
    return !quoted && /[.!?]/.test(char) ? ';' : char;
  }).join('');
  if (definition.collectiveObject === true) {
    const body=bounded.replace(/\{object\}/gi,'the proposed objects');
    const completion = definition.completionRequirement === 'all-choices-available' ? ', completing the action only if all choices were available' : '';
    return {...definition,expansion:`Perform the ${definition.keyword} action on {Object} as follows${completion}: [${body}]`};
  }
  return { ...definition, expansion: `Perform the ${definition.keyword} action as follows: ${bounded}.` };
}
