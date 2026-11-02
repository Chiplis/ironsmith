const escape = value => Array.from(String(value), character =>
  '\\^$.*+?()[]{}|'.includes(character) ? '\\' + character : character).join('');
function hasSingleActivationColon(text) {
  let depth = 0, quote = null, colons = 0;
  for (const character of text) {
    if (quote) { if (character === quote) quote = null; continue; }
    if (character === '"' || character === '“') { quote = character === '“' ? '”' : '"'; continue; }
    if (character === '{') { depth += 1; continue; }
    if (character === '}') { if (!depth) return false; depth -= 1; continue; }
    if (character === ':' && !depth) colons += 1;
  }
  return !depth && !quote && colons === 1;
}
// A single activation beginning with a self-transform instruction defines its
// action name. Neither a permanent marker nor another transform lends it one.
export function selfTransformActivationLabel(definition) {
  const name = String(definition.keyword || '').trim().toLowerCase();
  const reminder = String(definition.reminder || '');
  if (!/^[a-z]+$/.test(name) || /[\r\n]/.test(reminder) || !hasSingleActivationColon(reminder)) return null;
  if (!/^[^:]+:\s*Transform this(?: (?:permanent|creature|artifact|enchantment|land|planeswalker|battle))?\s*\./i.test(reminder)) return null;
  return name;
}
export function bindSelfTransformActivationLabel(text, definition) {
  const label = selfTransformActivationLabel(definition);
  if (!label || /\bThis is the [a-z-]+ activated ability\b/i.test(text)) return text;
  return `${String(text).trim().replace(/[.!?]+$/, '')}. This is the ${label} activated ability.`;
}
export function expandSelfTransformActionReferences(text, definition, sourceName) {
  const label = selfTransformActivationLabel(definition);
  if (!label || !sourceName) return text;
  const subject = `(?:${escape(sourceName)}|this (?:creature|permanent|artifact|enchantment|land|planeswalker|battle)|it)`;
  const pattern = new RegExp(`\\b(When|Whenever) you ${escape(definition.keyword)} ${subject}(?=\\s*,)`, 'gi');
  return text.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part, index) => index % 2 ? part
    : part.replace(pattern, (_, intro) => `${intro} this permanent transforms caused by its ${label} activated ability you activated`)).join('');
}
