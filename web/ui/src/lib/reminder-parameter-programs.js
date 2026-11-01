// A declaration's type operands can parameterize a whole existing action
// program. No operation or mechanic name is inferred here.
import { reminderParameters, parseParameterDeclaration, bindReminderParameters } from './reminder-costs.js';
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
const typeOnly = definition => {
  const parameters = reminderParameters(definition.reminder);
  return parameters.length === 1 && parameters[0] === 'type';
};
const plainType = value => /^[A-Za-z][A-Za-z'-]*(?:[ \t]+[A-Za-z][A-Za-z'-]*){0,2}$/.test(value)
  && !/\b(?:if|unless|then|while|until|times|where|for|each)\b/i.test(value);
const bodyFor = (definition, values, expand) => expand(bindReminderParameters(definition.reminder, definition.keyword, values)).trim();

// The selector and its anaphoric action are one local lexical binding.
// The executable form retains the resolution-time named choice; it is not a
// printed modal header, whose modes would be announced before resolution.
export function expandChosenTypeParameterPrograms(text, definition, expand = value => value) {
  const before = String(text);
  if (!typeOnly(definition)) return before;
  const pattern = new RegExp(`(^|[,.:;•]\\s*|\\bthen\\s+)choose[ \\t]+([^.!?;,\\n]+?)[ \\t]+or[ \\t]+([^.!?;,\\n]+?)[ \\t]*,[ \\t]*then[ \\t]+(${escape(definition.keyword)})(?![a-z])[ \\t]+an?[ \\t]+card[ \\t]+of[ \\t]+that[ \\t]+kind([.!?]?[ \\t]*$)`, 'gim');
  return before.replace(pattern, (whole, prefix, first, second, keyword, ending, offset) => {
    if (!outsideQuotation(before, offset + prefix.length)) return whole;
    const qualities = [first.trim(), second.trim()];
    if (qualities.some(quality => !plainType(quality) || /\b(?:and|or|not|chosen|that|this|kind|type)\b/i.test(quality))) return whole;
    if (qualities[0].toLowerCase() === qualities[1].toLowerCase()) return whole;
    const values = qualities.map(quality => parseParameterDeclaration(definition.reminder, keyword, `${keyword} a ${quality} card`, definition.template));
    if (values.some((value, index) => !value?.type || value.type.toLowerCase() !== qualities[index].toLowerCase())) return whole;
    const programs = values.map(value => bodyFor(definition, value, expand).replace(/[.!?]+$/, ''));
    return `${prefix}Choose ${qualities[0]} or ${qualities[1]}. If ${qualities[0]} was chosen, ${programs[0]}. Otherwise, ${programs[1]}${ending.trim() ? ending : '.'}`;
  });
}

export function expandTypeParameterPrograms(text, definition, expand = value => value) {
  const before = expandChosenTypeParameterPrograms(text, definition, expand);
  if (!typeOnly(definition)) return before;
  // Terminal declaration only: the native repeat header owns the complete
  // remainder of this ability. A later action cannot accidentally join it.
  const pattern = new RegExp(`(^|[,.:;•]\\s*|\\bthen\\s+)(${escape(definition.keyword)})(?![a-z])[ \\t]+([^.!?;\\n]+?)([.!?]?[ \\t]*$)`, 'gim');
  return before.replace(pattern, (whole, prefix, keyword, rest, ending, offset) => {
    if (!outsideQuotation(before, offset + prefix.length)) return whole;
    const repeated = rest.match(/^(.*?)\s+([XYZ]|\d+)\s+times(?:\s*,\s*where\s+([XYZ])\s+is\s+(.+))?$/i);
    if (repeated) {
      const [, declaration, count, variable, expression] = repeated;
      const symbolic = /^[XYZ]$/i.test(count);
      if (symbolic && (!expression || variable.toUpperCase() !== count.toUpperCase())) return whole;
      if (!symbolic && variable) return whole;
      if (expression && /\b(?:then|unless|otherwise)\b/i.test(expression)) return whole;
      const values = parseParameterDeclaration(definition.reminder, keyword, `${keyword} ${declaration}`, definition.template);
      if (!values?.type || !plainType(values.type)) return whole;
      const body = bodyFor(definition, values, expand);
      if (/\b[XYZ]\b/.test(body)) return whole; // Do not capture another program's variables.
      const binding = expression ? `, where ${count.toUpperCase()} is ${expression.trim()}` : '';
      return `${prefix}Repeat the following process ${symbolic ? count.toUpperCase() : count} times${binding}. ${body.replace(/[.!?]+$/, '')}${ending.trim() ? ending : '.'}`;
    }
    const sequence = rest.match(/^(.*?)\s*,\s*then\s+do\s+the\s+same\s+for\s+(.+)$/i);
    if (!sequence) return whole;
    const initial = parseParameterDeclaration(definition.reminder, keyword, `${keyword} ${sequence[1]}`, definition.template);
    if (!initial?.type || !plainType(initial.type)) return whole;
    // Comma-separated values with an ordinary final conjunction. Every value
    // independently substitutes the same already-validated layout.
    const tail = sequence[2].replace(/,\s*and\s+/gi, ', ').split(/\s*,\s*|\s+and\s+/i).map(value => value.trim());
    if (!tail.length || tail.some(value => !plainType(value))) return whole;
    const values = [initial.type, ...tail];
    const programs = values.map(type => bodyFor(definition, { ...initial, type }, expand).replace(/[.!?]+$/, ''));
    return prefix + programs.join('. ') + (ending.trim() ? ending : '.');
  });
}
export const hasTypeParameterProgram = (text, definition) => expandTypeParameterPrograms(text, definition) !== String(text);
