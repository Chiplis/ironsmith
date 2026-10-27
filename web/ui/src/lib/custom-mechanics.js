import { expandKnownReminderDefinition } from './official-reminder-definitions.js';
import { hasReminderMaterialParameter, bindReminderMaterials, hasExileCastDefinition, bindExileCastObject, hasNextUntapCostDefinition, nextUntapCostText, hasReminderCardParameter, bindReminderCard, isActivationReminder, bindActivationReminder, bindReminderCost, hasReminderCostReference, bindReminderActivationLabel, bindReminderValueFromDeclaration, hasRepeatedCopyPayment, expandRepeatedCopyPayment, bindReminderCostFromDeclaration } from './reminder-costs.js';
import { MTG_KEYWORD_RULES } from './mtg-keyword-rules.generated.js';
import { OFFICIAL_KEYWORD_FORMS, officialSurfaceForms, customKeywordForms, normalizeFormOverrides } from './keyword-forms.js';

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
  );
}

export function validateMechanic(definition) {
  const keyword = mechanicKey(definition.keyword);
  if (!/^[a-z][a-z '-]{0,63}$/.test(keyword) || official.has(keyword)) {
    throw new Error(`"${keyword}" cannot be defined as a custom mechanic. Use a distinct custom keyword; official keywords cannot be redefined.`);
  }
  const reminder = String(definition.reminder || '').trim();
  if (!reminder || reminder.length > 2000 || /[()]/.test(reminder)) {
    throw new Error(`"${keyword}" needs one unambiguous parenthetical reminder explaining its action.`);
  }
  const overrides = normalizeFormOverrides(definition.forms);
  const forms = customKeywordForms({ keyword, forms: overrides });
  for (const form of [forms.participle, forms.designation, forms.costNoun]) {
    if (form && form !== keyword && reservedForms.has(form)) {
      throw new Error(`"${keyword}" cannot use "${form}" as one of its forms; it belongs to an official keyword. Override it in the mechanic's forms.`);
    }
  }
  return { keyword, reminder, expansion: mechanicExpansion(bindReminderActivationLabel(reminder, keyword)), sourceCard: String(definition.sourceCard || 'an earlier card'), version: 1, ...(overrides ? { forms: overrides } : {}) };
}

export function mechanicConflict(existing, incoming) {
  return new Error(`Custom mechanic "${incoming.keyword}" could not be added: "${existing.sourceCard}" already defined it as "${existing.reminder}". This card defines it as "${incoming.reminder}". Definitions are retained even when a card fails to compile. Use the existing reminder text, or rename this mechanic everywhere in the card to give it a different meaning.`);
}

export function mergeMechanics(existing, incoming) {
  const definitions = new Map(existing.map(value => { const def = validateMechanic(value); return [def.keyword, def]; }));
  for (const value of incoming) {
    const def = validateMechanic(value), previous = definitions.get(def.keyword);
    if (previous && reminderKey(previous.reminder) !== reminderKey(def.reminder)) throw mechanicConflict(previous, def);
    if (!previous) definitions.set(def.keyword, def);
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
  return mergeMechanics(supplied, declared).filter(def => draft.faces.some(face => mechanicPattern(def).test(String(face.oracleText || '').replace(/\([^()]*\)/g, ''))));
}

export function compileDraftInput(draft, definitions) {
  return { ...draft, customMechanics: definitions, faces: draft.faces.map(face => {
    let text = String(face.oracleText || '');
    const semantic = text.replace(/\([^()]*\)/g, '');
    const used = definitions.filter(def => mechanicPattern(def).test(semantic) || valuePattern(def.keyword).test(semantic));
    if (!used.length) {
      const result = { ...face };
      delete result.customKeywordIdentities;
      return result;
    }
    // Source presentation is carried explicitly, never reconstructed by
    // replacing coincidentally similar compiler output.
    const displayOracleText = stripReminders(text).split('\n').map(line => line.trim()).filter(Boolean).join('\n');
    text = displayOracleText;
    for (const def of used) {
      let before = text;
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
    return { ...face, oracleText: text, displayOracleText,
      customKeywordIdentities: used.map(def => def.keyword) };
  }) };
}

const OPTIONAL_ADDITIONAL_COST = /\b(?:As an additional cost to cast this spell, you may|You may pay an additional)\b[^.]*/gi;
const PARTICIPLE_SUBJECTS = '(it|this spell|this creature|this permanent|this card)';

// A participle reference ("if it was blinked") to a custom optional additional
// cost tests that payment, the generic reading of "kicked". It binds only when
// the expanded face has exactly one non-repeatable optional additional cost;
// any other reference is left as written for strict compilation to judge.
function bindParticipleReferences(text, def) {
  const { participle } = customKeywordForms(def);
  const pattern = () => new RegExp(`\\b${PARTICIPLE_SUBJECTS}\\s+(was|wasn't|was not)\\s+${escape(participle)}\\b`, 'gi');
  if (!pattern().test(text)) return text;
  const optional = text.match(OPTIONAL_ADDITIONAL_COST) || [];
  if (optional.length !== 1 || /\bany number of times\b/i.test(optional[0])) return text;
  if (!(def.expansion.match(OPTIONAL_ADDITIONAL_COST) || []).length) return text;
  const before = text;
  return text.replace(pattern(), (match, subject, verb, offset) => {
    if (!outsideQuotation(before, offset)) return match;
    const owner = /^this spell$/i.test(subject) ? "this spell's" : 'its';
    const head = subject[0] === subject[0].toUpperCase() ? owner[0].toUpperCase() + owner.slice(1) : owner;
    return `${head} additional cost ${/^was$/i.test(verb) ? 'was' : "wasn't"} paid`;
  });
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

export async function prepareCustomCard(draft, compile, { known = [], reserve = async () => {} } = {}) {
  let definitions = draftMechanics(draft, known);
  await reserve(definitions);
  for (let attempts = 0; attempts < 16; attempts++) {
    try {
      const preview = await compile(compileDraftInput(draft, definitions));
      if (!preview?.canCreate) throw new Error('This card must compile completely before it can be saved.');
      return { draft: { ...draft, customMechanics: definitions }, input: compileDraftInput(draft, definitions), preview };
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
export const activeCustomMechanics = () => [...active.values()];
