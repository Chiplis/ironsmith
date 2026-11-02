// A positional rule binds the ability paragraphs below its declaration. The
// definition supplies the recipient filter; the custom label is immaterial.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function expandFollowingAbilities(text, definition) {
  const rule = String(definition.reminder || '').trim().match(/^(Each .+?) has the following abilities\.$/i);
  if (!rule) return null;
  const lines = String(text).split('\n');
  const declaration = new RegExp(`^\\s*${escape(definition.keyword)}\\s*[.]?\\s*$`, 'i');
  const indexes = lines.map((line,index) => declaration.test(line) ? index : -1).filter(index => index >= 0);
  if (!indexes.length) return null;
  if (indexes.length !== 1) throw new Error('A following-abilities declaration must have one unambiguous position.');
  const index = indexes[0];
  const abilities = lines.slice(index + 1).map(line => line.trim()).filter(Boolean);
  if (!abilities.length) throw new Error('A following-abilities declaration requires at least one subsequent ability.');
  // Curly outer quotes preserve an ability's own ordinary straight quotes.
  lines[index] = `${rule[1]} has ${abilities.map(ability => `“${ability.replace(/[.]$/, '')}”`).join(' and ')}.`;
  return lines.join('\n');
}

// A donor's positional ability scope is bound before the generic cost reader
// loses the source paragraph position. The definition names its own excluded
// designation; this works for any independently authored keyword.
export function bindFollowingSourceAbilityReferences(expanded, keyword, sourceTail) {
  const reference=new RegExp(`the non[- ]${escape(keyword)} abilities printed below this ability`,'gi');
  if(!reference.test(expanded))return expanded;
  reference.lastIndex=0;
  const selfDeclaration=new RegExp(`^\\s*${escape(keyword)}(?=\\s|[—–.]|$)`,'i');
  const abilities=String(sourceTail).split('\n').map(line=>line.trim()).filter(line=>line&&!selfDeclaration.test(line));
  if(!abilities.length)throw new Error('A following-source ability reference requires subsequent non-designated abilities.');
  return expanded.replace(reference,abilities.map(ability=>`“${ability.replace(/[.]$/,'')}”`).join(' and '));
}
