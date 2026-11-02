import { customKeywordForms } from './keyword-forms.js';
import { expandStatusEventReferences } from './status-event-references.js';
import { bindReminderParameters, parseParameterDeclaration, reminderParameters } from './reminder-costs.js';

const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const selfAction = def => new RegExp(`\\b${escape(def.keyword)}\\s+(this (?:creature|permanent|artifact|enchantment|land|planeswalker|battle|card))\\b`, 'gi');
function explicitStatusName(def) {
  const names = [...String(def.reminder).matchAll(/\bgive\s+(?:this (?:creature|permanent|artifact|enchantment|land|planeswalker|battle|card)|target (?:creature|permanent)|the most recently exiled card|it)\s+the\s+([a-z][a-z '-]*?)\s+status\b/gi)]
    .map(match => match[1].toLowerCase());
  const unique = [...new Set(names)];
  return unique.length === 1 ? unique[0] : null;
}
export function isStatusMechanic(def) { return selfAction(def).test(def.reminder) || explicitStatusName(def) !== null; }
export function statusName(def) { const forms = customKeywordForms(def); return explicitStatusName(def) || forms.designation || forms.participle; }
export function mentionsStatus(text, def) {
  return isStatusMechanic(def) && new RegExp(`\\b(?:${escape(def.keyword)}|${escape(statusName(def))})\\b`, 'i').test(text);
}
export function expandStatusActions(text, def) {
  if (def.perObjectProgram || !isStatusMechanic(def)) return text;
  return text.replace(selfAction(def), (_, subject) => `give ${subject} the ${statusName(def)} status`);
}

// Grants must contain the expanded ability, not merely an unknown keyword.
// The granted trigger retains "this creature", so it belongs to each recipient.
export function expandStatusGrants(text, def, expand) {
  if (!isStatusMechanic(def)) return text;
  const pattern = new RegExp(`\\b(has|have|gains?|and)\\s+${escape(def.keyword)}(?:\\s+((?:\\{[^{}]+\\})+|\\d+))?(?=\\s*(?:[.,]|and\\b|until\\b|$))`, 'gi');
  return text.replace(pattern, (match, verb, cost) => {
    const price = /^\d+$/.test(cost) && reminderParameters(def.reminder).includes('cost') ? `{${cost}}` : cost;
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
  // The contraction is a state reference, but a quoted literal name is data.
  // Granted rule bodies are expanded in their own ordinary program scope.
  const contraction = new RegExp(`\\bit['’]s\\s+${form}\\b`, 'gi');
  text = text.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part, index) => index % 2 ? part
    : part.replace(contraction, `it is ${name}`)).join('');
  // These references also occur inside granted, quoted abilities.
  return expandStatusEventReferences(text, name)
    .replace(new RegExp(`\\battacks while ${form}\\b`, 'gi'), `with the ${name} status attacks`)
    .replace(new RegExp(`\\b(?:isn't|is not) ${form}\\b`, 'gi'), `does not have the ${name} status`)
    .replace(new RegExp(`\\b(?:becomes|become) ${form}\\b`, 'gi'), `gains the ${name} status`)
    .replace(new RegExp(`\\b(?:is|are) ${form}\\b`, 'gi'), `has the ${name} status`)
    .replace(new RegExp(`\\b(?:a|an) ${form} (creatures?|permanents?|artifacts?|enchantments?|lands?|planeswalkers?|battles?|cards?|spells?)\\b`, 'gi'), (_,noun) => `${/^[aeiou]/i.test(noun)?'an':'a'} ${noun} with the ${name} status`)
    .replace(new RegExp(`\\b${form} (creatures?|permanents?|artifacts?|enchantments?|lands?|planeswalkers?|battles?|cards?|spells?)\\b`, 'gi'), `$1 with the ${name} status`);
}
