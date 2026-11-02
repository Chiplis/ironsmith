import { expandCollectionReferences } from './reminder-collection-references.js';
// Only an explicitly declared resolving-operation price reference is rewritten.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outside = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
function pattern(definition) {
  const reference = definition.operationPaymentAlternative;
  const verb = definition.forms?.eventVerb, label = definition.forms?.eventLabel;
  if (!reference || !verb || !label) return null;
  return new RegExp(`\\bYou[ \\t]+may[ \\t]+([^.!?"“”\\n]+?)[ \\t]+to[ \\t]+${escape(verb)}[ \\t]+rather[ \\t]+than[ \\t]+${escape(reference.defaultPhrase)}(?=[.!?]|$)`, 'gi');
}
export function hasOperationPaymentAlternative(text, definition) {
  const matcher = pattern(definition);
  return matcher ? [...String(text).matchAll(matcher)].some(match => outside(String(text), match.index)) : false;
}
export function expandOperationPaymentAlternatives(text, definitions) {
  let result = String(text);
  for (const definition of definitions) {
    const matcher = pattern(definition);
    if (!matcher) continue;
    const before = result;
    result = before.replace(matcher, (original, price, offset) => outside(before, offset)
      ? `You may pay the cost "${price.trim()}" rather than the cost of performing the ${definition.forms.eventLabel} action`
      : original);
  }
  return result;
}

// A source's declared action verb scopes a normal creation-time ability grant.
export function expandOperationTokenGrants(text, definitions) {
  let result = String(text);
  for (const definition of definitions) {
    const verb = definition.forms?.eventVerb, label = definition.forms?.eventLabel;
    if (!verb || !label || definition.actionProgram !== true) continue;
    const pattern = new RegExp(`\\bAs[ \\t]+you[ \\t]+${escape(verb)}[ \\t]+a[ \\t]+token,[ \\t]+it[ \\t]+gains[ \\t]+["“]([^"”]+)["”]`, 'gi');
    const before = result;
    result = before.replace(pattern, (original, body, offset) => outside(before, offset)
      ? `As you perform the ${label} action, tokens you create gain "${expandCollectionReferences(body, definitions)}"`
      : original);
  }
  return result;
}
