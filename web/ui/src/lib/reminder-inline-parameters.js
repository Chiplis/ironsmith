import { reminderParameters } from './reminder-costs.js';
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export const hasInlineParameters = definition => {
  const names = reminderParameters(definition.reminder);
  return names.length > 0 && names.every(name => name === 'number' || name === 'type');
};
export const inlineParameterPattern = definition => new RegExp(
  `(^|[,.:;•]\\s*|\\b(?:then|and)\\s+)(${escape(definition.keyword)})[ \\t]+([^.!?;,\\n]+?)(?=[ \\t]*(?:[,.;\\n]|$))([ \\t]*,[ \\t]*where[ \\t]+[XYZ][ \\t]+is[ \\t]+(?:(?!,[ \\t]*then\\b)[^.!?;\\n])+)?([ \\t]*,[ \\t]*then[ \\t]+)?`, 'gim');
// A type operand is a selector, never a control-flow tail. Declaration parsing
// remains owned by the existing Number/Type binder.
export const hasControlFlowOperand = values => values?.type && /\b(?:if|unless|then|while|until|times|where)\b/i.test(values.type);
const COUNTS = { again: 1, once: 1, twice: 2, one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9, ten: 10 };
export const repeatedActionPattern = definition => new RegExp(
  `(^|[,.:;•]\\s*|\\b(?:then|and)\\s+)(${escape(definition.keyword)})[ \\t]+(again|once|twice|(?:one|two|three|four|five|six|seven|eight|nine|ten|X|\\d+)[ \\t]+times)(?=[ \\t]*(?:[,.;\\n]|$))`, 'gim');
export function expandRepeatedActionDeclarations(text, definition) {
  return String(text).replace(repeatedActionPattern(definition), (match, prefix, _keyword, repetition, offset) => {
    if ((String(text).slice(0, offset).match(/["“”]/g) || []).length % 2) return match;
    const amount = repetition.toLowerCase().replace(/\s+times$/, '');
    if (/^x$/i.test(amount)) {
      let quoted = false;
      const body = String(definition.expansion || definition.reminder).trim().replace(/[.!?]+$/, '');
      // A repeated additional cost must remain a single payable cost with a
      // scaled amount, rather than becoming a spell effect process.
      const prior = String(text).slice(String(text).lastIndexOf('\n', offset) + 1, offset);
      if (/\bas an additional cost to cast\b/i.test(prior)) {
        const counted = body.match(/^(exile exactly) (one|two|three|four|five|six|seven|eight|nine|ten|\d+) (cards?\b[^.!?;\n]*)$/i);
        if (!counted) return match;
        return prefix + `${counted[1]} ${counted[2]} times X ${counted[3]}`;
      }
      const bounded = [...body].map(char => {
        if (/["“”]/.test(char)) quoted = !quoted;
        return !quoted && /[.!?]/.test(char) ? ';' : char;
      }).join('');
      return (/^and\s+$/i.test(prefix) ? '. ' : prefix) + `Repeat the following process X times: ${bounded}`;
    }
    const count = COUNTS[amount] ?? Number(amount);
    // Fixed finite repetition unrolls the complete action body, including
    // every optional decision and its fallback, rather than its last clause.
    if (!Number.isInteger(count) || count < 1 || count > 32) return match;
    const body = String(definition.expansion || definition.reminder).trim().replace(/[.!?]+$/, '');
    return (/^and\s+$/i.test(prefix) ? '. ' : prefix) + Array.from({ length: count }, () => body).join('. ');
  });
}

export function isSinglePlayerAction(definition) {
  const body = String(definition.expansion || definition.reminder).trim();
  if (!/^(?:create|draw|gain|lose|discard|sacrifice|return|exile|put|tap|untap)\b/i.test(body)) return false;
  let quoted = false, ends = 0;
  for (let index = 0; index < body.length; index++) {
    const char = body[index];
    if (/["“”]/.test(char)) quoted = !quoted;
    else if (!quoted && /[.!?;\n]/.test(char) && body.slice(index + 1).trim()) ends++;
  }
  return ends === 0;
}
const playerActionVerbPattern = definition => {
  const [head,...tail]=String(definition.keyword).split(/\s+/);
  const inflected=/[^aeiou]y$/i.test(head)?head.slice(0,-1)+'ies':/(?:s|sh|ch|x|z|o)$/i.test(head)?head+'es':head+'s';
  return `(?:${escape(definition.keyword)}|${escape([inflected,...tail].join(' '))})`;
};
export const playerActionPattern = definition => new RegExp(
  `((?:^|[,.:;]\\s*|\\b(?:then|and)\\s+)(?:you|they|that player|each player|each opponent|its controller|its owner|target player)[ \\t]+(?:may[ \\t]+)?)(${playerActionVerbPattern(definition)})(?=[ \\t]*(?:[,.;\\n]|$))`, 'gim');
// A bounded coordinated clause can elide the same explicit player subject.
// No subject is inferred across another clause or a quoted literal name.
function carryCoordinatedPlayerAction(text, definition) {
  const source=String(text);
  const pattern=new RegExp(`(^|[,.;]\\s*)(you|they|that player)[ \\t]+((?:discards?|draws?|sacrifices?|exiles?|reveals?|taps?|untaps?|puts?|gains?|loses?)[ \\t]+[^,.;\\n]+),[ \\t]*then[ \\t]+(${playerActionVerbPattern(definition)})(?=[ \\t]*(?:[,.;\\n]|$))`,'gim');
  return source.replace(pattern,(match,prefix,actor,clause,keyword,offset)=>{
    if ((source.slice(0,offset).match(/["“”]/g)||[]).length%2) return match;
    let quoted=false,unquoted='';
    for (const character of clause) {
      if (/["“”]/.test(character)) {quoted=!quoted;unquoted+=' ';}
      else unquoted+=quoted?' ':character;
    }
    // The regex can stop at a comma inside a quoted card name. Its entire
    // coordinator and keyword must be unquoted before any actor is carried.
    if (quoted) return match;
    // A conjunction may start a second explicit player instruction even
    // without a comma. The initial subject is then no longer carry evidence.
    for (const fragment of unquoted.split(/\b(?:and|then|or|but)\b/i).slice(1)) {
      const verb=fragment.search(/\b(?:discards?|draws?|sacrifices?|exiles?|reveals?|taps?|untaps?|puts?|gains?|loses?|creates?|pays?|takes?|casts?)\b/i);
      const subject=verb<0?fragment:fragment.slice(0,verb);
      if (/\b(?:you|they|players?|opponents?|owners?|controllers?|teammates?|teams?|everyone)\b/i.test(subject)) return match;
    }
    return `${prefix}${actor} ${clause}, then ${actor} ${keyword}`;
  });
}
export function expandSimplePlayerActionDeclarations(text, definition) {
  if (!isSinglePlayerAction(definition)) {
    const optional = new RegExp(`(^|[,.:;]\\s*|\\bthen\\s+)you[ \\t]+may[ \\t]+(${escape(definition.keyword)})(?=[ \\t]*(?:[,.;\\n]|$))`, 'gim');
    return String(text).replace(optional, (match, prefix, _keyword, offset) => {
      if ((String(text).slice(0, offset).match(/["“”]/g) || []).length % 2) return match;
      let quoted = false;
      const body = String(definition.expansion || definition.reminder).trim().replace(/[.!?]+$/, '');
      const bounded = [...body].map(char => {
        if (/["“”]/.test(char)) quoted = !quoted;
        return !quoted && /[.!?]/.test(char) ? ';' : char;
      }).join('');
      return prefix + `You may do the following: ${bounded}`;
    });
  }
  const carried=carryCoordinatedPlayerAction(text,definition);
  return carried.replace(playerActionPattern(definition), (match, prefix, _keyword, offset) => {
    if ((carried.slice(0, offset).match(/["“”]/g) || []).length % 2) return match;
    return prefix + String(definition.expansion || definition.reminder).trim().replace(/[.!?]+$/, '');
  });
}
