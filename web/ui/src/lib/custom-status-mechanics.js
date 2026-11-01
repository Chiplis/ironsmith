import { customKeywordForms } from './keyword-forms.js';
import { bindReminderParameters, parseParameterDeclaration, reminderParameters } from './reminder-costs.js';

const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const selfAction = def => new RegExp(`\\b${escape(def.keyword)}\\s+(this (?:creature|permanent|artifact|enchantment|land|planeswalker|battle|card))\\b`, 'gi');
export function isStatusMechanic(def) { return selfAction(def).test(def.reminder); }
export function statusName(def) { const forms = customKeywordForms(def); return forms.designation || forms.participle; }
export function mentionsStatus(text, def) {
  return isStatusMechanic(def) && new RegExp(`\\b(?:${escape(def.keyword)}|${escape(statusName(def))})\\b`, 'i').test(text);
}
export function expandStatusActions(text, def) {
  if (!isStatusMechanic(def)) return text;
  return text.replace(selfAction(def), (_, subject) => `give ${subject} the ${statusName(def)} status`);
}

// Grants must contain the expanded ability, not merely an unknown keyword.
// The granted trigger retains "this creature", so it belongs to each recipient.
export function expandStatusGrants(text, def, expand) {
  if (!isStatusMechanic(def)) return text;
  const pattern = new RegExp(`\\b(has|have|gains?|and)\\s+${escape(def.keyword)}(?:\\s+((?:\\{[^{}]+\\})+|\\d+))?(?=\\s*(?:[.,]|and\\b|$))`, 'gi');
  return text.replace(pattern, (match, verb, cost) => {
    const price = /^\d+$/.test(cost) ? `{${cost}}` : cost;
    const values = !cost && !reminderParameters(def.reminder).length ? {}
      : parseParameterDeclaration(def.reminder, def.keyword, `${def.keyword} ${price || ''}`, def.template);
    if (!values) throw new Error(`"${def.keyword}" requires its declared parameters when granted.`);
    const ability = expandStatusActions(expand(bindReminderParameters(def.reminder, def.keyword, values, customKeywordForms(def))), def);
    return `${verb} "${ability}"`;
  });
}

export function expandStatusReferences(text, def) {
  if (!isStatusMechanic(def)) return text;
  const name = statusName(def), form = escape(name);
  // These references also occur inside granted, quoted abilities.
  return text
    .replace(new RegExp(`\\battacks while ${form}\\b`, 'gi'), `with the ${name} status attacks`)
    .replace(new RegExp(`\\b(?:isn't|is not) ${form}\\b`, 'gi'), `does not have the ${name} status`)
    .replace(new RegExp(`\\b(?:becomes|become) ${form}\\b`, 'gi'), `gains the ${name} status`)
    .replace(new RegExp(`\\b(?:is|are) ${form}\\b`, 'gi'), `has the ${name} status`)
    .replace(new RegExp(`\\b${form} (creatures?|permanents?|artifacts?|enchantments?|lands?|planeswalkers?|battles?)\\b`, 'gi'), `$1 with the ${name} status`);
}
