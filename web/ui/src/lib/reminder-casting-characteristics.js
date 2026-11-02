// The declaration supplies the alternative mana cost and optionally size;
// independent reminder operands bind to those characteristics wherever used.
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export const hasPrintedCastingCharacteristics = reminder => /\bprinted alternative mana cost\b/i.test(reminder);
export function expandPrintedCastingCharacteristics(text, definition, sourceName = '') {
 if (!hasPrintedCastingCharacteristics(definition.reminder)) return text;
 const declaration = new RegExp(`^([ \\t]*)${escape(definition.keyword)}[ \\t]+((?:\\{[^{}]+\\})+)(?:[ \\t]*[—–-][ \\t]*(-?\\d+)\\/(-?\\d+))?[ \\t]*$`,'gim');
 const source = sourceName ? `(?:${escape(sourceName)}|this (?:spell|card))` : 'this (?:spell|card)';
 const resolution = new RegExp(`^If ${source} was cast for its ${escape(definition.keyword)} cost, it enters as a ([a-z][a-z '-]*) creature after it resolves\.[ \t]*$`, 'gim');
 const extra = [];
 const remaining = String(text).replace(resolution, (_line,types) => {
  extra.push(`When cast this way, after resolving this spell enters the battlefield as a ${types} creature.`);
  return '';
 });
 let expanded = false;
 const result = remaining.replace(declaration, (_line,prefix,cost,power,toughness) => {
  if (expanded) throw Error('One declared characteristic set is required to bind its selected resolution instructions.');
  expanded = true;
  let reminder=String(definition.reminder)
   .replace(/its printed alternative mana cost/gi,cost)
   .replace(/the printed alternative mana cost, its corresponding colors/gi,`mana cost ${cost}, colors determined by that mana cost`)
   .replace(/, and the printed alternative power and toughness if present/gi,power===undefined?'':`, and base power and toughness ${power}/${toughness}`);
  if (/\bprinted alternative\b/i.test(reminder)) throw Error('Unbound printed alternative characteristic.');
  return prefix+[reminder,...extra].join(' ');
 });
 return expanded ? result : text;
}
