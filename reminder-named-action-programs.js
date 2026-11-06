// A program declaration is explicit metadata, separate from a completed marker.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
const replacementPattern = definition => new RegExp(`\\b(If[ \\t]+(?:you|an opponent|a player)[ \\t]+would)[ \\t]+${escape(definition.keyword)}(?=[ \\t]*,)`, 'gi');

export function expandNamedActionReplacementReferences(text, definitions) {
  let result = String(text);
  const planned = /\bIf[ \t]+(?:you|an opponent|a player)[ \t]+would[ \t]+perform[ \t]+the[ \t]+([a-z][a-z '-]{0,63})[ \t]+action(?=[ \t]*,)/gi;
  for (const match of result.matchAll(planned)) {
    if (!outsideQuotation(result, match.index)) continue;
    const definition = definitions.find(definition => definition.keyword.toLowerCase() === match[1].trim().toLowerCase());
    if (definition?.actionProgram !== true) throw new Error(`"${match[1]}" needs an actionProgram declaration before its proposed operation can be replaced.`);
  }
  for (const definition of definitions) {
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
  if (/^Perform the .+ action as follows:/i.test(definition.expansion)) return definition;
  const body = String(definition.expansion || definition.reminder).trim().replace(/[.!?]+$/, '');
  let quoted = false;
  const bounded = [...body].map(char => {
    if (/["“”]/.test(char)) quoted = !quoted;
    return !quoted && /[.!?]/.test(char) ? ';' : char;
  }).join('');
  return { ...definition, expansion: `Perform the ${definition.keyword} action as follows: ${bounded}.` };
}
