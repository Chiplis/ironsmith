// Typed operands for inline custom actions. This module only substitutes a
// declared layout; strict native compilation remains the semantic validator.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const ABILITY = '(?:first strike|double strike|deathtouch|defender|flying|haste|hexproof|indestructible|lifelink|menace|reach|trample|vigilance)';
const PARAMETER = /\{(object|ability|name|player)\}/i;
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
export const hasActionParameters = definition => PARAMETER.test(String(definition.reminder || '')) || /\{(?:object|ability|name|player)\}/i.test(String(definition.template || ''));

export function validateActionTemplate(keyword, template, reminder) {
  const layout = String(template || '');
  const names = [...layout.matchAll(/\{([^{}]+)\}/g)].map(match => match[1].toLowerCase());
  if (!names.length || names.some(name => !['object', 'ability', 'name', 'player'].includes(name))) throw new Error(`"${keyword}"'s action layout needs {Object}, {Ability}, {Name}, and/or {Player}.`);
  if (names.includes('player') && names.length !== 1) throw new Error(`"${keyword}"'s player action layout must bind one player operand.`);
  if (names.includes('object') && names.includes('name')) throw new Error(`"${keyword}"'s action layout cannot ambiguously combine {Object} and {Name}.`);
  if (new Set(names).size !== names.length) throw new Error(`"${keyword}"'s action layout repeats an operand.`);
  // Mana symbols in the effect body are literals, not declaration operands.
  const used = [...String(reminder).matchAll(/\{(object|ability|name|player)\}/gi)].map(match => match[1].toLowerCase());
  if (used.some(name => !names.includes(name)) || names.some(name => !used.includes(name))) throw new Error(`"${keyword}"'s action layout and reminder must use the same operands.`);
  if (/[^a-z{}\s—–-]/i.test(layout)) throw new Error(`"${keyword}"'s action layout may contain only operands, words, spaces and dashes.`);
  if (/\}\{/.test(layout)) throw new Error(`"${keyword}"'s action operands need a space between them.`);
}
function actionPatternSource(definition) {
  let rest = String(definition.template || '').trim();
  if (!rest) return null;
  let source = '', position = 0;
  for (const match of rest.matchAll(/\{(object|ability|name|player)\}/gi)) {
    source += escape(rest.slice(position, match.index)).replace(/\s+/g, '[ \\t]+');
    const name = match[1].toLowerCase();
    source += name === 'player' ? '(?<player>target player|that player|each player|each opponent|its controller|its owner)'
      : name === 'ability' ? `(?<ability>${ABILITY})`
      : name === 'name' ? '(?<name>"[^"\\n]+"|“[^”\\n]+”|[^.!?;,:\\n"“”()]+?)'
      : '(?<object>[^.!?;,:\\n"“”()]+?)';
    position = match.index + match[0].length;
  }
  source += escape(rest.slice(position)).replace(/\s+/g, '[ \\t]+');
  return `(^|[,.:;•]\\s*|\\bthen\\s+)(${escape(definition.keyword)})[ \\t]+${source}(?:[ \\t]+(?<repetition>once|twice|(?:one|two|three|four|five|six|seven|eight|nine|ten|\\d+)[ \\t]+times))?(?=[ \\t]*(?:[.;\\n]|,\\s*then\\b|$))(?<then>[ \\t]*,[ \\t]*then[ \\t]+)?`;
}
export function actionParameterPattern(definition) {
  return new RegExp(actionPatternSource(definition) || '(?!)', 'gim');
}
function validObject(value, sourceName) {
  const selector = /^(?:it|them|this (?:creature|permanent|artifact|enchantment|land|source)|that (?:creature|permanent|artifact|enchantment|land)|target (?:creature|permanent|artifact|enchantment|land)(?: you (?:control|don't control))?)$/i;
  const targetSelector = /^(?:up to one\s+)?target\s+/i.test(value)
    && /\b(?:creature|permanent|artifact|enchantment|land)\b/i.test(value)
    && value.split(/\s+/).length <= 24
    && !/\b(?:if|unless|then|while|until|where|when|whenever|may|choose|create|draw|put|sacrifice)\b/i.test(value);
  return selector.test(value) || targetSelector || (sourceName && value.toLowerCase() === sourceName.toLowerCase());
}
export function expandActionParameterDeclarations(text, definition, sourceName = '') {
  return String(text).replace(actionParameterPattern(definition), (...args) => {
    const groups = args.at(-1), original = args[0], prefix = args[1];
    const offset = args.at(-3);
    if (!outsideQuotation(String(text), offset)) return original;
    // An action used as a casting/activation cost is not an effect macro.
    const keywordOffset = offset + prefix.length;
    const sentencePrefix = String(text).slice(Math.max(String(text).lastIndexOf('\n', keywordOffset - 1), String(text).lastIndexOf('.', keywordOffset - 1)) + 1, keywordOffset);
    if (/\b(?:as (?:an? )?(?:additional )?cost|costs? to (?:cast|activate))\b/i.test(sentencePrefix)) return original;
    const object = groups.object?.trim();
    if (object && !validObject(object, sourceName)) return original;
    const name = groups.name?.trim();
    if (name && (name.length > 200 || (!/^["“]/.test(name) && /\b(?:if|unless|then|while|until|where)\b/i.test(name)))) return original;
    if (groups.repetition && !groups.player) return original;
    const countWords = {once:1,twice:2,one:1,two:2,three:3,four:4,five:5,six:6,seven:7,eight:8,nine:9,ten:10};
    const repetition = groups.repetition?.toLowerCase().replace(/\s+times$/, '');
    const count = repetition ? (countWords[repetition] ?? Number(repetition)) : 1;
    if (!Number.isInteger(count) || count < 1 || count > 32) return original;
    const player = groups.player?.trim().toLowerCase();
    const quantified = /^each /.test(player || '');
    const values = { object, ability: groups.ability?.trim(), name, player: player === 'target player' || quantified ? 'that player' : player }; 
    const expanded = String(definition.reminder).replace(/\{(object|ability|name|player)\}/gi, (_, name) => values[name.toLowerCase()]);
    const body = expanded.trim().replace(/[.!?]+$/, '');
    // Quantified bodies stay inside one participant's scope. A multi-sentence
    // body needs a native scope boundary before it can be repeated safely.
    if (quantified && /[.!?;\n]/.test(body)) return original;
    const iterations = Array.from({length:count}, () => body);
    const program = quantified
      ? `For ${player}, ${iterations.join(', then ')}`
      : (player === 'target player' ? 'Choose target player. ' : '') + iterations.join('. ');
    return prefix + program + (groups.then ? '. Then ' : '');
  });
}
