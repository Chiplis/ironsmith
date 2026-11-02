// A passive action in a trigger head denotes gaining a designation. The
// same words in an intervening condition denote its current state instead.
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function expandStatusEventReferences(text, name) {
  const passive = new RegExp(`\\b(is|are)\\s+${escape(name)}\\b`, 'gi');
  return text.replace(/\b(When|Whenever)\s+([^,\n]+)(?=,)/gi,
    (match, trigger, head) => `${trigger} ${head.replace(passive, (_, verb) => `${verb.toLowerCase() === 'are' ? 'gain' : 'gains'} the ${name} status`)}`);
}
