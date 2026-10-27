// A named cost is a parameter of a mechanic's definition, not a keyword alias.
// Payment parsing remains the compiler's job, including all non-mana costs.
import { officialFormsWithCostNoun } from './keyword-forms.js';

const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
// `forms` (keyword-forms.js) names the cost noun when it differs from the
// keyword ("its upkeep cost" for Cumulative upkeep, or a custom override).
const costNouns = (keyword, forms) => [...new Set([forms?.costNoun, keyword].filter(Boolean).map(String))];
const reference = (keyword, forms) => new RegExp(`\\b(?:(?:its|the)\\s+(?:${costNouns(keyword, forms).map(escape).join('|')})|this)\\s+cost\\b`, 'gi');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;

export function hasReminderCostReference(reminder, keyword, forms) {
  // "Mana cost" always denotes the card's printed mana cost, never a slot.
  if (costNouns(keyword, forms).some(noun => noun.trim().toLowerCase() === 'mana')) return false;
  return [...String(reminder).matchAll(reference(keyword, forms))].some(match => outsideQuotation(reminder, match.index));
}

function declaredPrice(reminder, declaredCost) {
  const cost = String(declaredCost || '').trim().replace(/\.$/, '').trim();
  const qualified = cost.match(/^onto\s+(.+?)\s+(?=\{|(?:pay|discard|sacrifice|exile|return|tap|remove)\b)(.+)$/i);
  if (!qualified) return cost;
  const quality = qualified[1].trim();
  if (!new RegExp(`\\bAs you cast (?:a|an) ${escape(quality)} spell\\b`, 'i').test(reminder)) {
    throw new Error(`The declared spell quality "${quality}" must match the mechanic's definition.`);
  }
  return qualified[2].trim();
}

export function bindReminderCost(reminder, keyword, declaredCost, forms) {
  const cost = declaredPrice(reminder, declaredCost);
  if (!hasReminderCostReference(reminder, keyword, forms)) return reminder;
  if (!cost) throw new Error(`"${keyword}" requires a declared cost to resolve "its ${costNouns(keyword, forms)[0]} cost".`);
  if (/[\r\n()]/.test(cost) || /\bits\s+.+?\s+cost\b/i.test(cost)) {
    throw new Error(`"${keyword}" requires a concrete cost, including any non-mana payments.`);
  }
  const pattern = new RegExp(`(?:\\bpay\\s+)?${reference(keyword, forms).source}`, 'gi');
  return String(reminder).replace(pattern, (match, offset) => {
    if (!outsideQuotation(reminder, offset)) return match;
    const hadPay = /^pay\s+/i.test(match);
    const actionPrice = /^(?:pay|exile|discard|sacrifice|return|tap|remove)\b/i.test(cost);
    return `${hadPay && !actionPrice ? match.match(/^pay\s+/i)[0] : ''}${cost}`;
  });
}

// Audit/import helper: bind only a reference whose name matches an explicit
// declaration, and only when the declaration starts with a concrete payment.
export function bindReminderCostFromDeclaration(reminder, declaration) {
  const materials = bindReminderMaterials(reminder, declaration);
  if (materials.binding) return materials;
  const exile = bindExileCastFromDeclaration(reminder, declaration);
  if (exile.binding) return exile;
  const untap = bindNextUntapCost(reminder, declaration);
  if (untap.binding) return untap;
  const card = bindReminderCardFromDeclaration(reminder, declaration);
  if (card.binding) return card;
  const activation = bindActivationReminder(reminder, declaration);
  if (activation.binding) return activation;
  for (const match of String(reminder).matchAll(/\b(?:its|the)\s+([a-z][a-z '-]{0,63}?)\s+cost\b/gi)) {
    const keyword = match[1].trim();
    if (keyword.toLowerCase() === 'mana' || !outsideQuotation(reminder, match.index)) continue;
    // The cost noun names its keyword through the lexicon ("upkeep" ->
    // "cumulative upkeep"); an unlisted noun is its own (custom) name.
    const names = [...officialFormsWithCostNoun(keyword).map(forms => forms.name), keyword];
    const declared = names.map(name => String(declaration).match(new RegExp(`^(?:[a-z][a-z '-]*?\\s+)?${escape(name)}(?=[ \\t—–-]|$)(?:[ \\t]+\\d+[ \\t]*[—–-])?[ \\t—–-]*(.+)$`, 'i'))).find(Boolean);
    if (!declared) continue;
    const cost = declaredPrice(reminder, declared[1]);
    if (!/^(?:\{|(?:pay|exile|discard|sacrifice|return|tap|remove)\b|\d+\s+life\b)/i.test(cost)) continue;
    const bound = bindReminderCost(reminder, keyword, cost);
    // The paid-copy shorthand still omits its repeated payment once bound.
    if (repeatedCopyPayment(bound)) return { text: expandRepeatedCopyPayment(bound, keyword, cost), binding: { keyword, cost, expanded_full_rule: true } };
    return { text: bound, binding: { keyword, cost } };
  }
  const copy = repeatedCopyPayment(reminder);
  const declared = String(declaration).match(/^([a-z][a-z '-]*?)(?:[ \t]*[—–][ \t]*|[ \t]+(?=\{))(.+?)\.?$/i);
  if (declared && /\bthis cost\b/i.test(reminder) && hasReminderCostReference(reminder, declared[1])) {
    return { text: bindReminderCost(reminder, declared[1], declared[2]), binding: { keyword: declared[1], cost: declared[2] } };
  }
  if (copy && declared && copy.price.toLowerCase() === declared[2].trim().toLowerCase()) {
    return { text: expandRepeatedCopyPayment(reminder, declared[1], declared[2]), binding: { keyword: declared[1], cost: declared[2], expanded_full_rule: true } };
  }
  return { text: reminder, binding: null };
}

// A definition may name its own activation in a timing sentence. Resolve
// that self-reference, just like its named price, without granting identity.
export function bindReminderActivationLabel(reminder, keyword) {
  const pattern = new RegExp(`\\b${escape(keyword)}\\s+only\\s+as\\s+a\\s+sorcery\\b`, 'gi');
  return String(reminder).replace(pattern, (match, offset) =>
    outsideQuotation(reminder, offset) ? 'Activate only as a sorcery' : match);
}

// A numeric declaration can define a variable independently of the mechanic's
// name. Carry that explicit definition into its executable reminder, retaining
// the expression for the compiler to evaluate at resolution.
export function bindReminderValueFromDeclaration(reminder, declaration) {
  const binding = String(declaration).match(/,\s*where\s+([XYZ])\s+is\s+([^.!?]+)\.?\s*$/i);
  if (!binding || !outsideQuotation(declaration, binding.index)) return { text: reminder, binding: null };
  const variable = binding[1].toUpperCase(), expression = binding[2].trim();
  if (!new RegExp(`\\b${variable}\\b`, 'i').test(String(declaration).slice(0, binding.index))) return { text: reminder, binding: null };
  if (new RegExp(`\\bwhere\\s+${variable}\\s+is\\b`, 'i').test(reminder)) return { text: reminder, binding: null };
  for (const sentence of String(reminder).matchAll(/[^.!?]+[.!?]?/g)) {
    const offset = sentence[0].search(new RegExp(`\\b${variable}\\b`, 'i'));
    if (offset < 0 || !outsideQuotation(reminder, sentence.index + offset)) continue;
    const body = sentence[0].replace(/[.!?]$/, '');
    const replacement = `${body}, where ${variable} is ${expression}.`;
    return {
      text: reminder.slice(0, sentence.index) + replacement + reminder.slice(sentence.index + sentence[0].length),
      binding: { variable, expression },
    };
  }
  return { text: reminder, binding: null };
}

// A complete paid-copy reminder leaves the repeated additional payment in its
// declaration. Restore that payment as ordinary rules prose, using no keyword ID.
function repeatedCopyPayment(reminder) {
  const match = String(reminder).trim().match(/^When you cast this spell, copy it for each time you paid ([^.]+)\.\s*(You may choose new targets for the copies\.)(?:\s*(Copies become tokens\.))?$/i);
  if (!match || /["“”]/.test(match[1]) || !/^(?:\{|pay\b|exile\b|discard\b|sacrifice\b|return\b|tap\b|remove\b)/i.test(match[1])) return null;
  return { price: match[1].trim(), tail: `${match[2]}${match[3] ? ` ${match[3]}` : ''}` };
}
export const hasRepeatedCopyPayment = reminder => !!repeatedCopyPayment(reminder);

export function expandRepeatedCopyPayment(reminder, keyword, cost) {
  const shape = repeatedCopyPayment(reminder);
  if (!shape) return reminder;
  const price = bindReminderCost(`its ${keyword} cost`, keyword, cost);
  const payment = /^(?:pay|exile|discard|sacrifice|return|tap|remove)\b/i.test(price) ? price : `pay ${price}`;
  return `As an additional cost to cast this spell, you may ${payment} any number of times. When you cast this spell, copy it for each time this spell's additional cost was paid. ${shape.tail}`;
}

// Activation reminders modify the declared cost/effect rather than replacing
// it. This binding is independent of the label (Boast, Exhaust, or a custom
// name), and must not smuggle that label into native activation metadata.
export function isActivationReminder(reminder) {
  return /^Activate\b[^.!?]+\.\s*$/i.test(String(reminder).trim());
}

export function bindActivationReminder(reminder, declaration) {
  if (!isActivationReminder(reminder)) return { text: reminder, binding: null };
  const declared = String(declaration).trim().match(/^([a-z][a-z '-]*?)\s*[—–]\s*(.+)$/i);
  if (!declared || /[\r\n]/.test(declared[2])) return { text: reminder, binding: null };
  const keyword = declared[1];
  let activation = declared[2];
  const colon = activation.indexOf(':');
  if (colon < 1 || !activation.slice(colon + 1).trim() || !outsideQuotation(activation, colon)) return { text: reminder, binding: null };
  let restriction = String(reminder).trim();
  const selfReference = new RegExp(`^Activate (?:each|this) ${escape(keyword)} ability only once\\.$`, 'i');
  if (selfReference.test(restriction)) restriction = 'Activate this ability only once.';
  else if (!/^Activate (?:this ability )?only\b/i.test(restriction)) return { text: reminder, binding: null };
  // The printed upkeep/reveal reminder abbreviates a lasting public
  // reveal. Expand that complete shape to the full rule, for any label.
  if (/^Activate (?:this ability )?only during your upkeep and only once each turn\.$/i.test(restriction)
      && /\bReveal this card from your hand(?=\s*[:,])/i.test(activation)) {
    activation = activation.replace(/\bReveal this card from your hand(?=\s*[:,])/i, 'Reveal this card from your hand until upkeep ends');
    restriction = "Activate only during this card's owner's upkeep and only once each turn.";
  }
  return {
    text: `${activation.trim().replace(/\.$/, '')}. ${restriction}`,
    binding: { keyword, activation: activation.trim(), expanded_full_rule: true },
  };
}

// A formal card operand is distinct from the definition's later "it", which
// denotes the object actually entering the battlefield (a new incarnation).
export function hasReminderCardParameter(reminder) {
  return /^To [a-z][a-z '-]*? a card,\s*put it onto the battlefield face down\b/i.test(String(reminder).trim());
}

export function bindReminderCard(reminder, operand) {
  if (!hasReminderCardParameter(reminder)) return reminder;
  if (!/^(?:the top card of your library|one of them|one of those cards)$/i.test(String(operand).trim())) {
    throw new Error('This card definition needs an explicit supported card operand.');
  }
  return String(reminder).trim().replace(/^To [a-z][a-z '-]*? a card,\s*put it\b/i, `Put ${operand.trim()}`);
}

export function bindReminderCardFromDeclaration(reminder, declaration) {
  if (!hasReminderCardParameter(reminder)) return { text: reminder, binding: null };
  const keyword = String(reminder).trim().match(/^To ([a-z][a-z '-]*?) a card,/i)[1];
  const pattern = new RegExp(`\\b${escape(keyword)}\\s+(the top card of your library|one of them|one of those cards)(?=[,.;]|\\s+(?:and|then)\\b|$)`, 'gi');
  const match = [...String(declaration).matchAll(pattern)].find(match => outsideQuotation(declaration, match.index));
  return match ? { text: bindReminderCard(reminder, match[1]), binding: { keyword, operand: match[1] } } : { text: reminder, binding: null };
}

// A cost that commits this permanent to miss its next untap. The printed
// participle describes the resulting state; it does not give a keyword ID.
export const hasNextUntapCostDefinition = reminder => /^(?:An? [a-z][a-z '-]*? creature|This creature) (?:won't|will not) untap during your next untap step\.$/i.test(String(reminder).trim());
export const nextUntapCostText = 'Choose to have this creature not untap during your next untap step';
export function bindNextUntapCost(reminder, declaration) {
  if (!hasNextUntapCostDefinition(reminder)) return { text: reminder, binding: null };
  const colon = String(declaration).indexOf(':');
  if (colon < 0) return { text: reminder, binding: null };
  const price = String(declaration).slice(0, colon);
  const match = price.match(/(?:^|,\s*)([a-z][a-z '-]*?) this creature\s*$/i);
  if (!match || !outsideQuotation(declaration, match.index)) return { text: reminder, binding: null };
  const start = match.index + match[0].indexOf(match[1]);
  return { text: declaration.slice(0, start) + nextUntapCostText + declaration.slice(colon), binding: { keyword: match[1] } };
}

export const hasExileCastDefinition = reminder => /^To [a-z][a-z '-]*? (?:a|an) [a-z]+, exile it\. While it's exiled, its owner may cast it for .+ rather than its mana cost\.$/i.test(String(reminder).trim());
export function bindExileCastObject(reminder, operand) {
  if (!hasExileCastDefinition(reminder)) return reminder;
  if (!String(operand).trim() || /[.!?():\n]/.test(operand)) throw new Error('The exile definition requires an explicit object phrase.');
  return String(reminder).replace(/^To [a-z][a-z '-]*? (?:a|an) [a-z]+, exile it/i, `Exile ${operand.trim()}`);
}
export function bindExileCastFromDeclaration(reminder, declaration) {
  if (!hasExileCastDefinition(reminder)) return { text: reminder, binding: null };
  const keyword = String(reminder).match(/^To ([a-z][a-z '-]*?) (?:a|an) [a-z]+,/i)[1];
  const pattern = new RegExp(`\\b${escape(keyword)}\\s+([^.!?\\n]+)`, 'gi');
  const match = [...String(declaration).matchAll(pattern)].find(match => outsideQuotation(declaration, match.index));
  if (!match) return { text: reminder, binding: null };
  const operand = match[1].split(/,\s*(?:then\s+)?/i)[0];
  return { text: bindExileCastObject(reminder, operand), binding: { keyword, operand } };
}

// A shortened counted object phrase refers to the explicit material clause
// in its declaration. Restore that filter; never turn it into unrestricted cards.
export const hasReminderMaterialParameter = reminder => /\bExile the (?:one|two|three|four|five|six|seven|eight|nine|ten|\d+)(?: or more)? from among other permanents you control and\/or cards in your graveyard/i.test(String(reminder));
export function bindReminderMaterials(reminder, declaration) {
  const hole = String(reminder).match(/\bExile the ((?:one|two|three|four|five|six|seven|eight|nine|ten|\d+)(?: or more)?) from among other permanents you control and\/or cards in your graveyard/i);
  if (!hole) return { text: reminder, binding: null };
  const named = String(declaration).match(/^([a-z][a-z '-]*?) with (.+?)\s+(?=\{|pay\b|discard\b|sacrifice\b)(.+)$/i);
  if (!named) return { text: reminder, binding: null };
  const material = named[2].trim();
  if (!material.toLowerCase().startsWith(hole[1].toLowerCase() + ' ')) throw new Error('The reminder material count must match its declaration.');
  const text = String(reminder).replace(hole[0], `Exile ${material} from among other permanents you control and/or cards in your graveyard`);
  return { text: bindReminderActivationLabel(text, named[1]), binding: { keyword: named[1], material } };
}
