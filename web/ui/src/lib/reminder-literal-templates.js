// A literal declaration layout is explicit source syntax, not an inferred
// action operand. Only the complete configured line may name this definition.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;

export function hasLiteralTemplate(definition) {
  return !!String(definition.template || '').trim() && !/[{}]/.test(definition.template);
}

export function literalTemplatePattern(definition) {
  const template = String(definition.template || '').trim();
  if (/[\r\n]/.test(template)) throw new Error('A literal declaration layout must fit on one line.');
  const parts = template.split(/([ \t]+|[—–-])/).filter(Boolean);
  const suffix = parts.map((part, index) => {
    if (/^[ \t]+$/.test(part)) {
      const adjacentDash = /^[—–-]$/.test(parts[index - 1] || '') || /^[—–-]$/.test(parts[index + 1] || '');
      return adjacentDash ? '[ \\t]*' : '[ \\t]+';
    }
    if (/^[—–-]$/.test(part)) return '[ \\t]*[—–-][ \\t]*';
    return escape(part);
  }).join('');
  const separator = /^[—–-]/.test(template) ? '[ \\t]*' : '[ \\t]+';
  return new RegExp(`(^[ \\t]*)${escape(definition.keyword)}(?![a-z])${separator}${suffix}[ \\t]*\\.?[ \\t]*$`, 'gim');
}

export function hasLiteralDeclaration(text, definition) {
  return [...String(text).matchAll(literalTemplatePattern(definition))]
    .some(match => outsideQuotation(String(text), match.index));
}

export function expandLiteralDeclarations(text, definition) {
  const before = String(text);
  return before.replace(literalTemplatePattern(definition), (match, prefix, offset) =>
    outsideQuotation(before, offset) ? prefix + String(definition.expansion).trim() : match);
}
