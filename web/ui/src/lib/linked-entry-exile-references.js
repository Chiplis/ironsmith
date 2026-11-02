// A supplied action verb denotes the successful authored entry-exile receipt.
// Parse its bounded filter operand before appending the historical relation.
const escape = value => Array.from(String(value), character =>
  '\\^$.*+?()[]{}|'.includes(character) ? '\\' + character : character).join('');
export function bindLinkedEntryExileReference(text, definition, sourceName) {
  if (!definition.linkedExileVerb) return text;
  if (definition.linkedExileScope === 'source') return bindPersistentSourceNouns(text, definition.linkedExileVerb, sourceName);
  const producer=definition.keyword ? ` by its ability with designation "${definition.keyword}"` : '';
  const subject = `(?:it|this (?:creature|permanent|card)|${escape(sourceName || 'this card')})`;
  // The card noun terminates the modifier list; it cannot be swallowed as an
  // adjective while searching a later instruction for a second card noun.
  const modifiers = '(?:(?!card\\b)[a-z]+(?:,?\\s+))*';
  const comparison = '(?:\\d+|[XYZ]|one|two|three|four|five|six|seven|eight|nine|ten)';
  const quantity = `(?:\\s+with\\s+(?:power|toughness|mana value)\\s+${comparison}(?:\\s+or\\s+(?:greater|less))?)?`;
  const operand = `((?:a|an)\\s+${modifiers}card${quantity})`;
  const boundary = '(?=\\s*(?:[,.;\\n]|and\\b|or\\b|$))';
  const pattern = new RegExp(`\\b${subject}\\s+${escape(definition.linkedExileVerb)}\\s+${operand}${boundary}`, 'gi');
  return text.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part, index) => index % 2 ? part
    : bindEntryNouns(part.replace(pattern, (_, filter) => `${filter} was exiled as this creature entered${producer}`), subject, definition.linkedExileVerb, producer)).join('');
}

function bindEntryNouns(text, subject, verb, producer) {
  const reference = 'the card exiled as this creature entered'+producer;
  let result = text.replace(new RegExp(`\\bthe card ${subject}\\s+${escape(verb)}\\b`, 'gi'), match => /^[A-Z]/.test(match) ? reference[0].toUpperCase()+reference.slice(1) : reference);
  // This paired stat clause explicitly names the same card twice. Do not bind
  // arbitrary later demonstratives, targets, printed names, or quoted text.
  const stat = new RegExp(`\\bthe ${escape(verb)} card['’]s power and toughness equal to that card['’]s toughness\\b`, 'gi');
  result = result.replace(stat, `the power of ${reference} and toughness equal to the toughness of ${reference}`);
  const noun = escape(reference);
  // A trailing filter qualifies the returned card; evaluate the authored
  // entry receipt before the optional action, with no unrelated exile lookup.
  result = result.replace(new RegExp(`you may put ${noun} into your hand if it['’]s (a[^.\\n]+card)\\.`, 'gi'),
    (_, filter) => `if there is ${filter} exiled as this creature entered${producer}, you may put ${reference} into your hand.`);
  // "Instead" is one optional alternative. It cannot become a sequential
  // return followed by a cast from hand. Existing optional modal instructions
  // retain the same zero/return/cast outcomes and ordinary cast decisions.
  result = result.replace(new RegExp(`you may put ${noun} into your hand\\. If that card is (an? [^.\\n]+card), you may instead cast it without paying its mana cost\\.`, 'gi'),
    (_, filter) => `you may choose one instruction — Put ${reference} into your hand | If there is ${filter} exiled as this creature entered${producer}, cast ${reference} without paying its mana cost.`);
  return result;
}


// A persistent source-linked exile collection is distinct from a historical
// as-entry receipt. The declaration supplies both the opaque participle and
// the explicit domain; neither a keyword nor a card name selects behavior.
function bindPersistentSourceNouns(text, verb, sourceName) {
  const subject = `(?:this (?:creature|permanent|artifact|enchantment|card)|${escape(sourceName || 'this permanent')})`;
  const noun = new RegExp(`\\bthe ${escape(verb)} card\\b|\\bthe card ${subject}\\s+${escape(verb)}\\b`, 'gi');
  return String(text).split(/("[^"\n]*"|“[^”\n]*”)/g).map((part, index) => index % 2 ? part
    : part.replace(noun, match => /^[A-Z]/.test(match) ? 'The card exiled with this permanent' : 'the card exiled with this permanent')).join('');
}
