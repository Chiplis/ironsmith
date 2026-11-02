// Source-collection vocabulary is explicit declaration data, never inferred
// from the mechanic's name or from an arbitrary past-tense action.
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function normalizeCollectionReferences(value) {
  if (value == null) return null;
  if (!Array.isArray(value) || !value.length || value.length > 16) throw new Error('Collection references must be a nonempty declared list.');
  const seen = new Set();
  return value.map(entry => {
    const participle = String(entry?.participle || '').trim().toLowerCase();
    const label = String(entry?.label || '').trim().toLowerCase();
    if (!/^[a-z][a-z '-]{0,63}$/.test(participle) || !/^[a-z][a-z '-]{0,63}$/.test(label)) throw new Error('A collection reference needs an explicit participle and collection label.');
    if (seen.has(participle)) throw new Error('A collection reference participle must be unambiguous.');
    seen.add(participle);
    return { participle, label };
  }).sort((a, b) => a.participle.localeCompare(b.participle));
}
export function expandCollectionReferences(text, definitions) {
  const declarations = new Map();
  for (const definition of definitions) for (const reference of definition.collectionReferences || []) {
    const previous = declarations.get(reference.participle);
    if (previous && previous !== reference.label) throw new Error(`Collection reference "${reference.participle}" has conflicting labels.`);
    declarations.set(reference.participle, reference.label);
  }
  let result = String(text);
  for (const [participle, label] of declarations) {
    const relation = new RegExp(`\\bcards?[ \\t]+${escape(participle)}[ \\t]+with[ \\t]+(?:it|this creature)\\b`, 'gi');
    const source = result;
    result = source.replace(relation, (match, offset) => (source.slice(0, offset).match(/["“”]/g) || []).length % 2 ? match
      : `${/^card\b/i.test(match) ? 'card' : 'cards'} in exile from the ${label} collection`);

    const pattern = new RegExp(`\\bcards?([ \\t]+)(?:(?:it|she|he|this creature)[ \\t]+has|(?:it|she|he)['’]s)[ \\t]+${escape(participle)}[ \\t]+in[ \\t]+exile\\b`, 'gi');
    const before = result;
    result = result.replace(pattern, (match, spacing, offset) => {
      if ((before.slice(0, offset).match(/["“”]/g) || []).length % 2) return match;
      return `${/^card\b/i.test(match) ? 'card' : 'cards'} in exile from the ${label} collection`;
    });
  }
  return result;
}
