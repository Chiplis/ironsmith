// An ability-word declaration labels an arbitrary program body. This reader
// changes only the heading; the native compiler owns every body operation.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
const pattern = definition => new RegExp(`(^[ \\t]*)(${escape(definition.keyword)})[ \\t]*[—–][ \\t]*([^\\n]*)$`, 'gim');
export function hasAbilityWordDeclaration(text, definition) {
  if (definition.declarationKind !== 'ability-word') return false;
  return [...String(text).matchAll(pattern(definition))].some(match => outsideQuotation(String(text), match.index));
}
export function expandAbilityWordPrograms(text, definition) {
  if (definition.declarationKind !== 'ability-word') return text;
  const before = String(text);
  const expanded = before.replace(pattern(definition), (whole, indent, _keyword, body, offset) => {
    if (!outsideQuotation(before, offset)) return whole;
    if (!body.trim()) throw new Error(`"${definition.keyword}" needs its complete ability body on the same line.`);
    return `${indent}${definition.expansion.replace(/[.!?]+$/, '')} — ${body}`;
  });
  if (expanded === before) return before;
  const rules = (definition.programSelectionRules || []).filter(rule => !expanded.split('\n').includes(rule));
  return rules.length ? `${expanded}\n${rules.join('\n')}` : expanded;
}
