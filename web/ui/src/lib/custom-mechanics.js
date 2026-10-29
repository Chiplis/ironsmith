import { expandKnownReminderDefinition } from './official-reminder-definitions.js';
import { hasReminderMaterialParameter, bindReminderMaterials, hasExileCastDefinition, bindExileCastObject, hasNextUntapCostDefinition, nextUntapCostText, hasReminderCardParameter, bindReminderCard, isActivationReminder, bindActivationReminder, bindReminderCost, hasReminderCostReference, bindReminderActivationLabel, bindReminderValueFromDeclaration, hasRepeatedCopyPayment, expandRepeatedCopyPayment, bindReminderCostFromDeclaration, hasReminderParameters, parseParameterDeclaration, bindReminderParameters, parameterExample, reminderParameters, templateParameters } from './reminder-costs.js';
import { MTG_KEYWORD_RULES } from './mtg-keyword-rules.generated.js';
import { OFFICIAL_KEYWORD_FORMS, officialSurfaceForms, customKeywordForms, normalizeFormOverrides, surfaceForms } from './keyword-forms.js';
import { applyCustomTokens, draftTokens } from './custom-tokens.js';

export const mechanicKey = value => String(value || '').normalize('NFKC').replace(/[‘’]/g, "'").replace(/\s+/g, ' ').trim().toLowerCase();
export const reminderKey = value => mechanicKey(value).replace(/[“”]/g, '"');
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const official = new Set([...MTG_KEYWORD_RULES.flatMap(rule => [rule.title, ...rule.aliases]), ...officialSurfaceForms()].map(mechanicKey));
// Words a custom form may not take: an official form would make a reference
// such as "if it was kicked" ambiguous between the two keywords.
const reservedForms = new Set([...official, ...OFFICIAL_KEYWORD_FORMS.map(forms => forms.costNoun).filter(Boolean), 'mana', 'additional', 'alternative']);

// Bounded, deterministic normalization only. Unsupported meanings still fail
// strict compilation; this is not a general natural-language code generator.
export function mechanicExpansion(reminder, declaration = '') {
  return String(expandKnownReminderDefinition(reminder, declaration)).trim().replace(
    /\b(spells you cast) cost (\{?\d+\}?) less to cast until end of turn\b/gi,
    (_, subject, amount) => `${subject} this turn cost {${amount.replace(/[{}]/g, '')}} less to cast`,
  ).replace(
    // "You may pay {Cost} to draw a card" means paying buys the effect; the
    // compiler reads that as "You may pay {Cost}. If you do, draw a card."
    /\b(you may pay) (\{cost\}|(?:\{[^{}]+\})+|its [a-z][a-z '-]* cost) to (?!cast\b)([^.!?]+)([.!?]|$)/gi,
    (_, may, price, effect) => `${may} ${price}. If you do, ${effect}.`,
  );
}

// The name may carry the card line's layout after the keyword ("Behold a
// {Type}", "Forge {Number} {Cost}"): the keyword is the text before the first
// placeholder, less an article that belongs to the layout; the rest is its
// declaration template.
export function splitMechanicName(name) {
  const text = String(name || '').normalize('NFKC').replace(/\s+/g, ' ').trim();
  const brace = text.indexOf('{');
  if (brace < 0) return { keyword: mechanicKey(text), template: null };
  const head = text.slice(0, brace).replace(/[\s—–-]+$/, '').replace(/\s+an?$/i, '');
  return { keyword: mechanicKey(head), template: text.slice(head.length).trim() || null };
}

function validateTemplate(keyword, template, reminder) {
  const named = templateParameters(template);
  const unknown = String(template).match(/\{(?!(?:cost|number|type)\})([^{}]*)\}/i);
  if (unknown) throw new Error(`"{${unknown[1]}}" is not a parameter. Use {Cost}, {Number} or {Type} in "${keyword}"'s layout.`);
  const repeated = named.find((name, index) => named.indexOf(name) !== index);
  if (repeated) throw new Error(`"${keyword}"'s layout names {${repeated[0].toUpperCase()}${repeated.slice(1)}} more than once.`);
  if (/[^a-z{}\s—–-]/i.test(String(template).replace(/\{[a-z]+\}/gi, ''))) throw new Error(`"${keyword}"'s layout may hold only its parameters, words, spaces and dashes.`);
  const gaps = String(template).split(/\{(?:cost|number|type)\}/i).slice(1, -1);
  if (gaps.some(gap => !gap)) throw new Error(`"${keyword}"'s layout needs a space or a dash between two parameters.`);
  const used = reminderParameters(reminder);
  const missing = used.find(name => !named.includes(name));
  if (missing) throw new Error(`The reminder uses {${missing[0].toUpperCase()}${missing.slice(1)}}, so "${keyword}"'s layout must place it.`);
  // A price may also be named as "its <keyword> cost".
  const unused = named.find(name => !used.includes(name) && !(name === 'cost' && hasReminderCostReference(reminder, keyword)));
  if (unused) throw new Error(`"${keyword}"'s layout places {${unused[0].toUpperCase()}${unused.slice(1)}}, but the reminder never uses it.`);
}

export function validateMechanic(definition) {
  const split = splitMechanicName(definition.keyword);
  const keyword = split.keyword;
  const template = split.template || (definition.template ? String(definition.template).trim() : null);
  if (!/^[a-z][a-z '-]{0,63}$/.test(keyword) || official.has(keyword)) {
    throw new Error(`"${keyword}" cannot be defined as a custom mechanic. Use a distinct custom keyword; official keywords cannot be redefined.`);
  }
  const reminder = String(definition.reminder || '').trim();
  if (!reminder || reminder.length > 2000 || /[()]/.test(reminder)) {
    throw new Error(`"${keyword}" needs one unambiguous parenthetical reminder explaining its action.`);
  }
  // Braces hold mana symbols or the declaration parameters.
  const unknown = reminder.match(/\{(?!(?:cost|number|type)\})([a-z][a-z ]{2,})\}/i);
  if (unknown) throw new Error(`"{${unknown[1]}}" is not a parameter. Use {Cost} for a price, {Number} for an amount and {Type} for a card type, as in "${keyword} 3—{1}{U}".`);
  if (template) validateTemplate(keyword, template, reminder);
  const overrides = normalizeFormOverrides(definition.forms);
  const forms = customKeywordForms({ keyword, forms: overrides });
  for (const form of [forms.participle, forms.designation, forms.costNoun, ...forms.variants]) {
    if (form && form !== keyword && reservedForms.has(form)) {
      throw new Error(`"${keyword}" cannot use "${form}" as one of its forms; it belongs to an official keyword. Override it in the mechanic's forms.`);
    }
  }
  return { keyword, reminder, expansion: mechanicExpansion(bindReminderActivationLabel(reminder, keyword)), sourceCard: String(definition.sourceCard || 'an earlier card'), version: 1, ...(template ? { template } : {}), ...(overrides ? { forms: overrides } : {}) };
}

// Every word or phrase that names a mechanic: its name and each form.
const namingForms = def => { const forms = customKeywordForms(def); return [...surfaceForms(forms), forms.costNoun]; };

// A second custom keyword may not reuse another's name or forms; references
// such as "if it was blinked" must name exactly one mechanic.
export function mechanicFormClash(definitions, incoming) {
  const words = new Set(namingForms(incoming));
  for (const def of definitions) {
    if (def.keyword === incoming.keyword) continue;
    const shared = namingForms(def).find(form => words.has(form));
    if (shared) return new Error(`Custom mechanic "${incoming.keyword}" cannot use "${shared}": "${def.keyword}" (from ${def.sourceCard}) already uses it. Give one of them a different form.`);
  }
  return null;
}

export function mechanicConflict(existing, incoming) {
  return new Error(`Custom mechanic "${incoming.keyword}" could not be added: "${existing.sourceCard}" already defined it as "${existing.reminder}". This card defines it as "${incoming.reminder}". Definitions are retained even when a card fails to compile. Use the existing reminder text, or rename this mechanic everywhere in the card to give it a different meaning.`);
}

export function mergeMechanics(existing, incoming) {
  const definitions = new Map(existing.map(value => { const def = validateMechanic(value); return [def.keyword, def]; }));
  for (const value of incoming) {
    const def = validateMechanic(value), previous = definitions.get(def.keyword);
    if (previous && reminderKey(previous.reminder) !== reminderKey(def.reminder)) throw mechanicConflict(previous, def);
    if (!previous) {
      const clash = mechanicFormClash([...definitions.values()], def);
      if (clash) throw clash;
      definitions.set(def.keyword, def);
    }
  }
  return [...definitions.values()];
}

function actionPattern(keyword) {
  // Restrict substitution to complete action clauses, never card names, quoted
  // grants, or a keyword embedded in another word. Costed standalone
  // declarations use mechanicPattern instead.
  return new RegExp(`(^|[,.:;]\\s*|\\bthen\\s+)(${escape(keyword)})(?=\\s*(?:[,.;(]|$))`, 'gim');
}

function costedPattern(keyword) {
  return new RegExp(`(^[ \t]*)(${escape(keyword)})(?=[ \t—–]|$)(?:[ \t]*[—–][ \t]*|[ \t]+)?([^\n]*)$`, 'gim');
}

function valuePattern(keyword) {
  return new RegExp(`(^[ \t]*)(${escape(keyword)})[ \t]+([XYZ],\\s*where\\s+[XYZ]\\s+is\\s+[^\\n]+)$`, 'gim');
}

function cardParameterPattern(keyword) {
  return new RegExp(`(^|[,.:;]\\s*|\\bthen\\s+)(${escape(keyword)})\\s+(the top card of your library|one of them|one of those cards)(?=\\s*(?:[,.;(]|$))`, 'gim');
}

function exileObjectPattern(keyword) {
  return new RegExp(`(^|[,.:;]\\s*|\\bthen\\s+)(${escape(keyword)})\\s+([^.!?(),:\\n]+)`, 'gim');
}

function selfCostPattern(keyword) {
  return new RegExp(`(^|,\\s*)(${escape(keyword)})\\s+this creature(?=\\s*:)`, 'gim');
}

function mechanicPattern(definition) {
  if (hasReminderParameters(definition.reminder, definition.template)) return costedPattern(definition.keyword);
  if (hasExileCastDefinition(definition.reminder)) return exileObjectPattern(definition.keyword);
  if (hasNextUntapCostDefinition(definition.reminder)) return selfCostPattern(definition.keyword);
  if (hasReminderCardParameter(definition.reminder)) return cardParameterPattern(definition.keyword);
  return hasReminderMaterialParameter(definition.reminder) || isActivationReminder(definition.reminder) || hasReminderCostReference(definition.reminder, definition.keyword, customKeywordForms(definition)) || hasRepeatedCopyPayment(definition.reminder)
    ? costedPattern(definition.keyword) : actionPattern(definition.keyword);
}

function stripReminders(text) {
  let depth = 0, result = '';
  for (const character of text) {
    if (character === '(') depth++;
    else if (character === ')') {
      if (!depth) throw new Error('Unbalanced reminder parentheses. Close each reminder before saving.');
      depth--;
    } else if (!depth || character === '\n') result += character;
  }
  if (depth) throw new Error('Unbalanced reminder parentheses. Close each reminder before saving.');
  return result;
}

function outsideQuotation(text, offset) {
  return (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
}

function definingReminders(face, keyword) {
  const lines = String(face.oracleText || '').split('\n');
  const found = [];
  for (let index = 0; index < lines.length; index++) {
    const semantic = lines[index].replace(/\([^()]*\)/g, '');
    if (![...semantic.matchAll(actionPattern(keyword)), ...semantic.matchAll(costedPattern(keyword)), ...semantic.matchAll(cardParameterPattern(keyword)), ...semantic.matchAll(selfCostPattern(keyword)), ...semantic.matchAll(exileObjectPattern(keyword))]
      .some(match => outsideQuotation(semantic, match.index))) continue;
    let next = index + 1;
    while (next < lines.length && !lines[next].trim()) next++;
    const adjacent = lines[index] + (/^\s*\(/.test(lines[next] || '') ? lines[next] : '');
    if (/\([^)]*\(/.test(adjacent)) throw new Error(`"${keyword}" needs a single reminder without nested parentheses; write its full meaning in that reminder.`);
    const reminders = [...adjacent.matchAll(/\(([^()]*)\)/g)];
    if (reminders.length > 1) throw new Error(`"${keyword}" has more than one possible reminder. Put its single definition in parentheses on the same line as the action, or immediately below it.`);
    if (reminders.length === 1) found.push({ keyword, reminder: reminders[0][1].trim(), sourceCard: face.name, sourceLine: index });
  }
  return found;
}

export function draftMechanics(draft, known = []) {
  const supplied = mergeMechanics(known, draft.customMechanics || []);
  const declared = supplied.flatMap(def => draft.faces.flatMap(face => definingReminders(face, def.keyword)));
  return mergeMechanics(supplied, declared).filter(def => draft.faces.some(face => {
    const semantic = String(face.oracleText || '').replace(/\([^()]*\)/g, '');
    return mechanicPattern(def).test(semantic) || designationPattern(def).test(semantic);
  }));
}

export function compileDraftInput(draft, definitions) {
  return { ...draft, customMechanics: definitions, faces: draft.faces.map(face => {
    let text = String(face.oracleText || '');
    const semantic = text.replace(/\([^()]*\)/g, '');
    const used = definitions.filter(def => mechanicPattern(def).test(semantic) || valuePattern(def.keyword).test(semantic));
    // A card naming another card's keyword ("a blinked spell") reads its
    // receipts but never gains its identity.
    const referenced = definitions.filter(def => designationPattern(def).test(semantic));
    if (!used.length && !referenced.length) {
      const result = { ...face };
      delete result.customKeywordIdentities;
      delete result.customKeywordPayments;
      return result;
    }
    // Source presentation is carried explicitly, never reconstructed by
    // replacing coincidentally similar compiler output.
    const displayOracleText = stripReminders(text).split('\n').map(line => line.trim()).filter(Boolean).join('\n');
    text = displayOracleText;
    for (const def of used) {
      let before = text;
      if (hasReminderParameters(def.reminder, def.template)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, keyword, rest, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          const values = parseParameterDeclaration(def.reminder, keyword, `${keyword} ${rest}`, def.template);
          if (!values) throw new Error(`"${keyword}" needs its parameters, for example "${parameterExample(def.reminder, keyword, def.template)}".`);
          return prefix + mechanicExpansion(bindReminderParameters(def.reminder, keyword, values, customKeywordForms(def)));
        });
        continue;
      }
      if (/^You may cast this spell with different mana cost, color, and size\. It keeps its abilities and types\.$/i.test(def.reminder)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, keyword, parameters, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          const expanded = mechanicExpansion(def.reminder, `${keyword} ${parameters}`);
          if (expanded === def.reminder) throw new Error(`"${keyword}" needs a mana cost and power/toughness, such as {2}{U}—2/3.`);
          return prefix + expanded;
        });
        continue;
      }

      if (hasReminderMaterialParameter(def.reminder)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, keyword, material, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          const bound = bindReminderMaterials(def.reminder, `${keyword} ${material}`);
          if (!bound.binding) throw new Error('The material definition needs its complete declaration.');
          return prefix + bound.text;
        });
        continue;
      }
      if (hasExileCastDefinition(def.reminder)) {
        const pattern = new RegExp(exileObjectPattern(def.keyword).source + '(\\s*,\\s*then\\s+)?', 'gim');
        text = text.replace(pattern, (match, prefix, _keyword, operand, then, offset) => outsideQuotation(before, offset) ? prefix + bindExileCastObject(def.reminder, operand).replace(/[.!?]+$/, '') + (then ? '. Then ' : '') : match);
        continue;
      }
      if (hasNextUntapCostDefinition(def.reminder)) {
        text = text.replace(selfCostPattern(def.keyword), (match, prefix, _keyword, offset) => outsideQuotation(before, offset) ? prefix + nextUntapCostText : match);
        continue;
      }
      if (hasReminderCardParameter(def.reminder)) {
        const pattern = new RegExp(cardParameterPattern(def.keyword).source + '(\\s*,\\s*then\\s+)?', 'gim');
        text = text.replace(pattern, (match, prefix, _keyword, operand, then, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          return prefix + bindReminderCard(def.expansion, operand).replace(/[.!?]+$/, '') + (then ? '. Then ' : '');
        });
        continue;
      }
      text = text.replace(valuePattern(def.keyword), (match, prefix, keyword, parameter, offset) => {
        if (!outsideQuotation(before, offset)) return match;
        const bound = bindReminderValueFromDeclaration(def.expansion, `${keyword} ${parameter}`);
        if (!bound.binding) throw new Error(`"${keyword}" requires a matching variable in its reminder and declaration.`);
        return prefix + bound.text;
      });
      before = text;
      if (isActivationReminder(def.reminder)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, keyword, activation, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          const bound = bindActivationReminder(def.reminder, `${keyword} — ${activation}`);
          if (!bound.binding) throw new Error(`"${keyword}" requires a complete cost and effect before its activation reminder.`);
          return prefix + mechanicExpansion(bound.text);
        });
        continue;
      }
      if (hasReminderCostReference(def.reminder, def.keyword, customKeywordForms(def)) || hasRepeatedCopyPayment(def.reminder)) {
        text = text.replace(costedPattern(def.keyword), (match, prefix, _word, cost, offset) => {
          if (!outsideQuotation(before, offset)) return match;
          return `${prefix}${hasRepeatedCopyPayment(def.reminder) ? expandRepeatedCopyPayment(def.expansion, def.keyword, cost) : bindReminderCost(def.expansion, def.keyword, cost, customKeywordForms(def))}`;
        });
        continue;
      }
      const pattern = new RegExp(actionPattern(def.keyword).source + '(\\s*,\\s*then\\s+)?', 'gim');
      text = text.replace(pattern, (match, prefix, _word, then, offset) => {
        if (!outsideQuotation(before, offset)) return match;
        return `${prefix}${def.expansion.replace(/[.!?]+$/, '')}${then ? '. Then ' : ''}`;
      });
    }
    for (const def of used) text = bindParticipleReferences(text, def);
    for (const def of referenced) text = bindDesignations(text, def);
    const payments = used.filter(def => definitionPayment(text, def)).map(def => def.keyword);
    const result = { ...face, oracleText: text, displayOracleText,
      customKeywordIdentities: used.map(def => def.keyword),
      ...(payments.length === 1 ? { customKeywordPayments: payments } : {}) };
    if (!used.length) delete result.customKeywordIdentities;
    if (payments.length !== 1) delete result.customKeywordPayments;
    return result;
  }) };
}

const OPTIONAL_ADDITIONAL_COST = /\b(?:As an additional cost to cast this spell, you may|You may pay an additional)\b[^.]*/gi;
// Both alternative-cost wordings: "You may cast this spell for {R} …" and
// "You may pay {R} rather than pay this spell's mana cost."
const ALTERNATIVE_PRICE = /\bYou may (?:cast this (?:spell|card)\b[^.]*?\bfor|pay) ((?:\{[^{}]+\})+)(?=[ .,])(?:(?<=\bfor (?:\{[^{}]+\})+)|(?= rather than pay this (?:spell|card)'s mana cost))/gi;
const ALTERNATIVE_DEFINITION = /\bYou may (?:cast this (?:spell|card)\b[^.]*?\bfor\b|pay\b[^.]*?\brather than pay this (?:spell|card)'s mana cost\b)/i;
// A timed exile's expanded special action, the described form of Suspend
// ("Forge 4—{B}{B}"), and the reminder shape that defines one.
const TIMED_EXILE = /\bIf you could begin to cast this card from your hand, you may pay (?:\{[^{}]+\})+ and exile it with \S+ [a-z][a-z -]* counters on it\. This action doesn't use the stack\b/gi;
const TIMED_EXILE_DEFINITION = /\bRather than cast this card from your hand, you may pay \S+ and exile it with \S+ [a-z][a-z -]* counters on it\b/i;
const PARTICIPLE_SUBJECTS = '(it|this spell|this creature|this permanent|this card)';

// The single generic payment a definition creates on an expanded face: an
// optional additional cost, an alternative price in mana, or a timed exile.
// Anything else (repeatable, ambiguous, non-mana) has no payment a form can name.
export function definitionPayment(text, def) {
  const optional = text.match(OPTIONAL_ADDITIONAL_COST) || [];
  const alternatives = [...text.matchAll(ALTERNATIVE_PRICE)];
  if (TIMED_EXILE_DEFINITION.test(def.reminder)) {
    if ((text.match(TIMED_EXILE) || []).length !== 1 || alternatives.length || optional.length) return null;
    return { kind: 'timed-exile' };
  }
  if (def.expansion.match(OPTIONAL_ADDITIONAL_COST)) {
    if (optional.length !== 1 || alternatives.length || /\bany number of times\b/i.test(optional[0])) return null;
    return { kind: 'additional' };
  }
  if (ALTERNATIVE_DEFINITION.test(def.expansion)) {
    if (alternatives.length !== 1 || optional.length) return null;
    return { kind: 'alternative', cost: alternatives[0][1] };
  }
  return null;
}

// A participle reference ("if it was blinked") tests the payment its definition
// created, the generic reading of "kicked" or "cast for its warp cost". It binds
// only to a unique payment; any other reference is left as written for strict
// compilation to judge.
function bindParticipleReferences(text, def) {
  const forms = customKeywordForms(def);
  const words = [forms.participle, ...forms.variants].map(escape).join('|');
  const pattern = () => new RegExp(`\\b${PARTICIPLE_SUBJECTS}\\s+(was|wasn't|was not)\\s+(?:${words})\\b`, 'gi');
  if (!pattern().test(text)) return text;
  const payment = definitionPayment(text, def);
  // A timed exile's later free cast has no price a participle could test.
  if (!payment || payment.kind === 'timed-exile') return text;
  const before = text;
  return text.replace(pattern(), (match, subject, verb, offset) => {
    if (!outsideQuotation(before, offset)) return match;
    const owner = /^this spell$/i.test(subject) ? "this spell's" : 'its';
    const head = subject[0] === subject[0].toUpperCase() ? owner[0].toUpperCase() + owner.slice(1) : owner;
    const price = payment.kind === 'additional' ? 'additional' : payment.cost;
    return `${head} ${price} cost ${/^was$/i.test(verb) ? 'was' : "wasn't"} paid`;
  });
}

// "a blinked spell": another spell paid with that keyword's payment. Rewritten
// to "a spell using blink", which the compiler reads as that receipt.
function designationPattern(def) {
  const forms = customKeywordForms(def);
  const words = [...new Set([forms.participle, forms.designation, ...forms.variants].filter(Boolean))].map(escape).join('|');
  return new RegExp(`\\b(${words})\\s+((?:[a-z]+\\s+){0,3}?spells?)\\b`, 'gi');
}

function bindDesignations(text, def) {
  const before = text;
  return text.replace(designationPattern(def), (match, _form, noun, offset) =>
    outsideQuotation(before, offset) ? `${noun} using ${def.keyword}` : match);
}

// If another line fails first, still retain an unambiguous bare custom
// action declaration from this attempted card. Never infer arbitrary sentences.
function otherDeclarations(draft, definitions) {
  const candidates = [];
  for (const face of draft.faces) {
    const semantic = String(face.oracleText || '').replace(/\([^()]*\)/g, '');
    for (const match of semantic.matchAll(/(?:^|[,.:;]\s*|\bthen\s+)([a-z][a-z '-]*?)\s+(the top card of your library|one of them|one of those cards)(?=\s*(?:[,.;]|$))/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasReminderCardParameter(declaration.reminder)) candidates.push(declaration);
      }
    }
    for (const match of semantic.matchAll(/^[ \t]*([a-z][a-z '-]*?)[ \t]+[XYZ],\s*where\s+[XYZ]\s+is\s+[^\n]+$/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (bindReminderValueFromDeclaration(declaration.reminder, match[0]).binding) candidates.push(declaration);
      }
    }
    for (const match of String(face.oracleText || '').matchAll(/\(To ([a-z][a-z '-]*?) (?:a|an) [a-z]+, exile it\./gi)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasExileCastDefinition(declaration.reminder)) candidates.push(declaration);
      }
    }
    for (const line of semantic.split('\n')) {
      const match = line.match(/(?:^|,\s*)([a-z][a-z '-]*?) this creature\s*:/i);
      if (!match) continue;
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasNextUntapCostDefinition(declaration.reminder)) candidates.push(declaration);
      }
    }
    for (const match of semantic.matchAll(/^([a-z][a-z '-]*?) with [^\n]+$/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasReminderMaterialParameter(declaration.reminder)) candidates.push(declaration);
      }
    }
    // A parameterized declaration: "Keyword 3—{1}{U}", "Keyword 3 {1}{U}",
    // "Keyword a Goblin" or "Keyword 3". The keyword is one of the line's
    // leading words, proven by a reminder that reads the rest as its values.
    for (const line of semantic.split('\n')) {
      const words = line.trim().split(/\s+/);
      // The shortest such keyword: "Rally an Elf" names "rally", not "rally an".
      for (let count = 1, found = false; !found && count < Math.min(words.length, 5); count++) {
        const keyword = mechanicKey(words.slice(0, count).join(' '));
        if (!/^[a-z][a-z '-]*$/.test(keyword) || official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
        for (const declaration of definingReminders(face, keyword)) {
          if (hasReminderParameters(declaration.reminder) && parseParameterDeclaration(declaration.reminder, keyword, line.trim())) {
            candidates.push(declaration);
            found = true;
          }
        }
      }
    }
    // A costed declaration has an explicit dash or starts its price with mana.
    // Do not infer arbitrary prose as a mechanic name.
    for (const match of semantic.matchAll(/^[ \t]*([a-z][a-z '-]*?)(?:[ \t]*[—–][ \t]*|[ \t]+(?=\{))[^\n]+$/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      for (const declaration of definingReminders(face, keyword)) {
        if (hasReminderCostReference(declaration.reminder, keyword) || bindReminderCostFromDeclaration(declaration.reminder, match[0]).binding) candidates.push(declaration);
      }
    }
    for (const match of semantic.matchAll(/(?:^|[,.:;]\s*|\bthen\s+)([a-z]+)(?=\s*(?:[,.;]|$))/gim)) {
      const keyword = mechanicKey(match[1]);
      if (official.has(keyword) || definitions.some(def => def.keyword === keyword)) continue;
      candidates.push(...definingReminders(face, keyword));
    }
  }
  return candidates;
}

export async function prepareCustomCard(draft, compile, { known = [], reserve = async () => {}, knownTokens = [] } = {}) {
  let definitions = draftMechanics(draft, known);
  // Custom tokens the card names travel with it, like its mechanics.
  const tokens = draftTokens(draft, knownTokens);
  const { customTokens: _carried, ...rest } = draft;
  const prepared = { ...rest, ...(tokens.length ? { customTokens: tokens } : {}) };
  const compilerInput = defs => applyCustomTokens(compileDraftInput(prepared, defs), tokens);
  await reserve(definitions);
  for (let attempts = 0; attempts < 16; attempts++) {
    try {
      const preview = await compile(compilerInput(definitions));
      if (!preview?.canCreate) throw new Error('This card must compile completely before it can be saved.');
      return { draft: { ...prepared, customMechanics: definitions }, input: compilerInput(definitions), preview };
    } catch (error) {
      const keyword = mechanicKey(String(error?.message || error).match(/could not find verb in effect clause \(clause: '([^']+)'/)?.[1]);
      if (!keyword || definitions.some(def => def.keyword === keyword) || official.has(keyword) || !/^[a-z][a-z '-]{0,63}$/.test(keyword)) {
        const declarations = otherDeclarations(draft, definitions);
        if (declarations.length) {
          const byReminder = new Map();
          for (const declaration of declarations) {
            const key = `${declaration.sourceCard}:${declaration.sourceLine}`, prior = byReminder.get(key);
            if (prior && prior !== declaration.keyword) throw new Error('Multiple custom actions share one reminder. Give each action its own definition on a separate line.');
            byReminder.set(key, declaration.keyword);
          }
          definitions = mergeMechanics(definitions, declarations);
          await reserve(definitions);
          // Line-family and target diagnostics can hide the unknown action.
          // Newly discovered definitions deserve the same compile retry as a
          // missing-verb diagnostic; remaining errors still fail strictly.
          continue;
        }
        throw error;
      }
      const candidates = draft.faces.flatMap(face => definingReminders(face, keyword));
      if (!candidates.length) throw new Error(`${String(error?.message || error)} Add a parenthetical reminder defining "${keyword}" on its action line or immediately below it.`);
      if (draft.faces.some(face => {
        const newLines = new Set(definingReminders(face, keyword).map(candidate => candidate.sourceLine));
        return definitions.some(def => definingReminders(face, def.keyword).some(candidate => newLines.has(candidate.sourceLine)));
      })) {
        throw new Error(`Cannot tell which action the reminder defines. Give "${keyword}" its own reminder on a separate line.`);
      }
      definitions = mergeMechanics(definitions, candidates);
      // Persist immediately, even if the next compilation fails elsewhere.
      await reserve(definitions);
    }
  }
  throw new Error('Too many unresolved custom mechanics. Simplify the card and define each action separately.');
}

const active = new Map();
let revision = 0;
export const customMechanicRevision = () => revision;
export function installCustomMechanics(definitions) {
  let changed = false;
  for (const value of definitions) {
    const def = validateMechanic(value);
    if (JSON.stringify(active.get(def.keyword)) !== JSON.stringify(def)) {
      active.set(def.keyword, def);
      changed = true;
    }
  }
  if (changed) revision++;
}
export function uninstallCustomMechanic(keyword) {
  if (active.delete(mechanicKey(keyword))) revision++;
}
export const activeCustomMechanics = () => [...active.values()];

// A mechanic defined on its own (the Mechanics tab) has no card to prove it
// compiles. Compile it on a minimal card that uses it the way a card would,
// through the same expansion as a real draft. Shapes that need a
// card-specific declaration (materials, activations, variables) can only be
// judged on a real card.
const PROBE_COST = '{1}';
const PERMANENT_REMINDER = /\b(?:this (?:creature|permanent)|enters|attacks|blocks|dies|leaves the battlefield)\b/i;

function probeUsage(def) {
  const { keyword, reminder } = def;
  const name = keyword[0].toUpperCase() + keyword.slice(1);
  if (/^You may cast this spell with different mana cost/i.test(reminder) || hasReminderMaterialParameter(reminder)
    || hasExileCastDefinition(reminder) || hasNextUntapCostDefinition(reminder) || isActivationReminder(reminder)
    || /\b[XYZ]\b/.test(reminder)) return null;
  if (hasReminderParameters(reminder, def.template)) return parameterExample(reminder, name, def.template).replace('{1}{U}', PROBE_COST);
  if (hasReminderCardParameter(reminder)) return `${name} the top card of your library.`;
  if (hasReminderCostReference(reminder, keyword, customKeywordForms(def)) || hasRepeatedCopyPayment(reminder)) return `${name} ${PROBE_COST}`;
  return `${name}.`;
}

export async function probeMechanic(value, compile) {
  const def = validateMechanic(value);
  const usage = probeUsage(def);
  if (!usage) return { checked: false };
  const shapes = [
    { cardTypes: ['Instant'] },
    { cardTypes: ['Creature'], power: '2', toughness: '2' },
  ];
  if (PERMANENT_REMINDER.test(def.reminder)) shapes.reverse();
  const attempt = async shape => {
    const draft = { faces: [{ name: 'Mechanic Test', manaCost: '{0}', ...shape, oracleText: usage }] };
    const preview = await compile(compileDraftInput(draft, [def]));
    if (!preview?.canCreate) throw new Error('The reminder text did not compile completely.');
    return preview.faces?.[0]?.rawCompilation || '';
  };
  let firstError = null;
  for (const shape of shapes) {
    try {
      await attempt(shape);
      // A sentence with no ability shape ("You may pay {Cost} to …") compiles
      // as a spell instruction, which a permanent never runs.
      const onPermanent = await attempt(shapes.find(candidate => candidate.power) || shape).catch(() => '');
      return { checked: true, usage, permanentAbility: grantsPermanentAbility(onPermanent) };
    } catch (error) {
      firstError ||= error;
    }
  }
  const failure = new Error(String(firstError?.message || firstError));
  failure.usage = usage;
  throw failure;
}

// Abilities beyond the keyword's own identity marker.
const grantsPermanentAbility = raw => (raw.match(/\bAbility \{/g) || []).length > (raw.match(/\bKeywordMarker\b/g) || []).length;
