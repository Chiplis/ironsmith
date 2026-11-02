// A configured type operand can precede its keyword. The operand is still
// substituted into the author's reminder; no behavior is inferred from it.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
export const hasPrefixParameter = definition => definition.prefixTemplate === '{Type}';
export function expandPrefixParameters(text, definition) {
  const before = String(text);
  if (!hasPrefixParameter(definition)) return before;
  const pattern = new RegExp(`(^[ \\t]*)([A-Za-z][A-Za-z ,'-]*?)[ \\t]+${escape(definition.keyword)}[ \\t]*\\.?[ \\t]*$`, 'gim');
  return before.replace(pattern, (whole, prefix, operand, offset) => {
    if (!outsideQuotation(before, offset)) return whole;
    const types = operand.trim().replace(/,?\s+(?:and|or)\s+/gi, ', ').split(/\s*,\s*/);
    if (types.some(type => !/^[A-Za-z][A-Za-z'-]*$/.test(type)
      || /^(?:when|whenever|if|unless|then|has|have|gains?|with|without|this|that)$/i.test(type))) return whole;
    const normalized = types.length < 2 ? types[0]
      : types.length === 2 ? types.join(' or ') : types.slice(0, -1).join(', ') + ' or ' + types.at(-1);
    return prefix + definition.expansion.replace(/\{Type\}/gi, normalized);
  });
}
export const hasPrefixDeclaration = (text, definition) => expandPrefixParameters(text, definition) !== String(text);
