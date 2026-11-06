// Two independently executable abilities can be granted in the zones where
// each functions. The price reader binds costs; no card/mechanic names are fixed.
import {parseParameterDeclaration, bindReminderParameters} from './reminder-costs.js';
import {customKeywordForms} from './keyword-forms.js';
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
function components(definition, price) {
  const lines = String(definition.reminder || '').split('\n').map(line => line.trim()).filter(Boolean);
  if (lines.length !== 2 || !/^You may cast this spell for \{Cost\} rather than pay its mana cost\./i.test(lines[0])
      || !/^Any time .+?, you may pay \{Cost\} to take a special action\./i.test(lines[1])) return null;
  const values = parseParameterDeclaration(definition.reminder, definition.keyword, `${definition.keyword} ${price}`, definition.template);
  if (!values) return null;
  return lines.map(line => bindReminderParameters(line, definition.keyword, values, customKeywordForms(definition)));
}
export function expandCompoundCastingActionGrants(text, definitions) {
  let result = String(text);
  for (const definition of definitions) {
    const pattern = new RegExp(`^([^\n"“”]+?) (?:has|have) ${escape(definition.keyword)}[ \\t]+([^\n]+?)\\.?(?=$)`, 'gim');
    result = result.replace(pattern, (original, subject, price) => {
      const bound = components(definition, price.replace(/\.$/, '')); if (!bound) return original;
      const recipients = subject.split(/\s+and\s+(?=(?:attacking|blocking|tapped|untapped)\s+(?:creatures|permanents)\b)/i);
      const programs = [];
      for (const recipient of recipients) {
        if (/\bcards? in your hand$/i.test(recipient)) programs.push(`${recipient} have the alternative casting cost "${bound[0]}".`);
        else if (/\b(?:creatures|permanents) you control$/i.test(recipient)) programs.push(`${recipient} have "${bound[1]}".`);
        else return original;
      }
      return programs.join('\n');
    });
  }
  return result;
}
export function bindSourceCastingPriceReferences(text, definitions, sourceName) {
  let result = String(text);
  for (const definition of definitions) {
    const declaration = new RegExp(`^${escape(definition.keyword)}[ \\t—–-]+([^\n]+)$`, 'im').exec(result);
    if (!declaration) continue;
    const bound = components(definition, declaration[1].replace(/\.$/, '')); if (!bound) continue;
    const source = `(?:${escape(sourceName)}|this card)`;
    const reference = new RegExp(`You may cast ${source} from your (graveyard|hand|exile|library) for its ${escape(definition.keyword)} cost\\.`, 'gi');
    result = result.replace(reference, (_original, zone) => bound[0]
      .replace(/^You may cast this spell for /i, `You may cast this spell from your ${zone.toLowerCase()} for `));
  }
  return result;
}
