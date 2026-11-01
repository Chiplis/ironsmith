import { reminderParameters } from './reminder-costs.js';
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export const hasInlineParameters = definition => {
  const names = reminderParameters(definition.reminder);
  return names.length > 0 && names.every(name => name === 'number' || name === 'type');
};
export const inlineParameterPattern = definition => new RegExp(
  `(^|[,.:;•]\\s*|\\bthen\\s+)(${escape(definition.keyword)})[ \\t]+([^.!?;,\\n]+?)(?=[ \\t]*(?:[,.;\\n]|$))([ \\t]*,[ \\t]*where[ \\t]+[XYZ][ \\t]+is[ \\t]+(?:(?!,[ \\t]*then\\b)[^.!?;\\n])+)?([ \\t]*,[ \\t]*then[ \\t]+)?`, 'gim');
// A type operand is a selector, never a control-flow tail. Declaration parsing
// remains owned by the existing Number/Type binder.
export const hasControlFlowOperand = values => values?.type && /\b(?:if|unless|then|while|until|times|where)\b/i.test(values.type);
const COUNTS = { again: 1, once: 1, twice: 2, one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9, ten: 10 };
export const repeatedActionPattern = definition => new RegExp(
  `(^|[,.:;•]\\s*|\\bthen\\s+)(${escape(definition.keyword)})[ \\t]+(again|once|twice|(?:one|two|three|four|five|six|seven|eight|nine|ten|\\d+)[ \\t]+times)(?=[ \\t]*(?:[,.;\\n]|$))`, 'gim');
export function expandRepeatedActionDeclarations(text, definition) {
  return String(text).replace(repeatedActionPattern(definition), (match, prefix, _keyword, repetition, offset) => {
    if ((String(text).slice(0, offset).match(/["“”]/g) || []).length % 2) return match;
    const amount = repetition.toLowerCase().replace(/\s+times$/, '');
    const count = COUNTS[amount] ?? Number(amount);
    // Fixed finite repetition unrolls the complete action body, including
    // every optional decision and its fallback, rather than its last clause.
    if (!Number.isInteger(count) || count < 1 || count > 32) return match;
    const body = String(definition.expansion || definition.reminder).trim().replace(/[.!?]+$/, '');
    return prefix + Array.from({ length: count }, () => body).join('. ');
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
export const playerActionPattern = definition => new RegExp(
  `((?:^|[,.:;]\\s*|\\bthen\\s+)(?:you|they|that player|each player|each opponent|its controller|its owner|target player)[ \\t]+(?:may[ \\t]+)?)(${escape(definition.keyword)})(?=[ \\t]*(?:[,.;\\n]|$))`, 'gim');
export function expandSimplePlayerActionDeclarations(text, definition) {
  if (!isSinglePlayerAction(definition)) return text;
  return String(text).replace(playerActionPattern(definition), (match, prefix, _keyword, offset) => {
    if ((String(text).slice(0, offset).match(/["“”]/g) || []).length % 2) return match;
    return prefix + String(definition.expansion || definition.reminder).trim().replace(/[.!?]+$/, '');
  });
}
