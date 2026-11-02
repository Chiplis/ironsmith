// Cohort rows can describe different actions on the same source card. Import
// only that card's explicitly authored executable candidates, never a global
// keyword library or a display-name match.
import { mechanicKey, reminderKey, validateMechanic } from '../../web/ui/src/lib/custom-mechanics.js';
import { customKeywordForms } from '../../web/ui/src/lib/keyword-forms.js';

export const sourceIdentity = card => {
  for (const value of [card.source_card_key, card.id]) {
    if (typeof value === 'string' && value.trim()) return value.trim();
  }
  return null;
};
export function executableDefinitions(mechanic, card = null) {
  if (mechanic.support_status === 'unsupported_requires_new_rules') return [];
  const definitions = card?.executable_definitions ?? mechanic.executable_definitions ?? (mechanic.executable_reminder
    ? [{keyword: mechanic.executable_keyword || mechanic.keyword, reminder: mechanic.executable_reminder}] : []);
  // The immutable source row supplies import evidence, never rule semantics.
  return definitions.map(definition => definition.sourceKeyword
    ? {...definition,sourceReminder:definition.sourceReminder || mechanic.original_reminder}
    : definition);
}
export function indexSourceDefinitions(cohorts) {
  const index = new Map();
  for (const {path = '<memory>', fixture} of cohorts) {
    for (const mechanic of fixture.mechanics || []) {
      for (const card of mechanic.representative_cards || []) {
        const definitions = executableDefinitions(mechanic, card);
        if (!definitions.length) continue;
        const key = sourceIdentity(card);
        if (!key) continue;
        const rows = index.get(key) || [];
        rows.push({definitions, origin: `${path}:${mechanic.id || mechanic.keyword}`});
        index.set(key, rows);
      }
    }
  }
  return index;
}
function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]));
  return typeof value === 'string' ? mechanicKey(value) : value;
}
function fingerprint(definition) {
  return JSON.stringify(canonical({keyword: definition.keyword, reminder: reminderKey(definition.reminder), namedObjectReferences: definition.namedObjectReferences || [], collectiveObject: definition.collectiveObject === true, completionRequirement: definition.completionRequirement || null, resolutionProgramMultiplicity: definition.resolutionProgramMultiplicity || null, programSelectionRules: definition.programSelectionRules || [], triggerReferences: definition.triggerReferences || null, statusPredicate:definition.statusPredicate || null, costProgram: !!definition.costProgram, operationPaymentAlternative: definition.operationPaymentAlternative || null, paymentReferences: definition.paymentReferences || [],
    abilityDesignation: definition.abilityDesignation || '', objectRelation: definition.objectRelation || null, collectionReferences: definition.collectionReferences || [], paymentKind: definition.paymentKind || '', textAdditionVerb: definition.textAdditionVerb || '', linkedExileVerb: definition.linkedExileVerb || '', linkedExileScope: definition.linkedExileScope || 'entry', paymentReminder: definition.paymentReminder || '', declarationKind: definition.declarationKind || '', resultReference: definition.resultReference || '', template: definition.template || '', prefixTemplate: definition.prefixTemplate || '', appliesToSubtype: definition.appliesToSubtype || '', sourceScope: definition.sourceScope || '', numberedCharacteristics: !!definition.numberedCharacteristics, permanentLinkRelation: definition.permanentLinkRelation || '', sourceKeyword: definition.sourceKeyword || '', actionProgram: definition.actionProgram === true, perObjectProgram: definition.perObjectProgram === true, forms: customKeywordForms(definition)}));
}
export function definitionsForSource(card, ownDefinitions, index, ownOrigin = '<current row>') {
  const key = sourceIdentity(card);
  const candidates = [{definitions: ownDefinitions, origin: ownOrigin}, ...(key ? index.get(key) || [] : [])];
  const merged = new Map();
  for (const {definitions, origin} of candidates) {
    for (const raw of definitions) {
      // Validate lazily so an invalid unrelated card's candidate cannot change
      // this card's input. Every returned definition has passed validation.
      let definition;
      try {
        definition = validateMechanic(raw);
        if (raw.sourceKeyword) {
          if (!/^[a-z][a-z '-]{0,63}[!?]?$/i.test(raw.sourceKeyword)) throw new Error('A source keyword alias must be plain words with at most one terminal exclamation or question mark.');
          const reminders = raw.sourceReminders || [raw.sourceReminder];
          if (!reminders.length || reminders.some(reminder => typeof reminder !== 'string' || !reminder.trim())) throw new Error('A source keyword alias requires its exact original reminder evidence.');
          definition.sourceKeyword = raw.sourceKeyword;
          definition.sourceReminders = [...new Set(reminders)];
        }
      }
      catch (error) { throw new Error(`Invalid executable definition for source ${key || '<unkeyed>'} in ${origin}: ${error.message}`); }
      const keyword = mechanicKey(definition.keyword), signature = fingerprint(definition);
      const previous = merged.get(keyword);
      if (previous && previous.signature !== signature) {
        const error = new Error(`Conflicting executable definitions for source ${key || '<unkeyed>'}, keyword "${keyword}": ${previous.origin} versus ${origin}.`);
        error.code = 'same_source_definition_conflict';
        throw error;
      }
      if (previous && definition.sourceReminders) previous.definition.sourceReminders =
        [...new Set([...(previous.definition.sourceReminders || []),...definition.sourceReminders])];
      if (!previous) merged.set(keyword, {definition, signature, origin});
    }
  }
  return [...merged.values()].map(entry => entry.definition);
}
