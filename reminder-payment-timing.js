// A source-authored payment-window exception is read independently of its
// keyword declaration. Its native action program receives explicit timing.
const escape = value => String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const ACTION = /(^|\n)Any time you could cast a sorcery, you may pay \{Cost\} to take a special action\./gi;
export function bindSourcePaymentTiming(text, definitions, sourceName) {
  let result = String(text);
  const timingFacts = [];
  const rewritten = definitions.map(definition => {
    const program = String(definition.expansion || definition.reminder || '');
    if (!program.match(ACTION)) return definition;
    const subject = `(?:${escape(sourceName)}['’]s|This (?:card|creature|permanent)['’]s)`;
    const pattern = new RegExp(`^${subject} ${escape(definition.keyword)} cost can be paid any time you could cast an instant\\.\\s*$`, 'gim');
    let observed = false;
    result = result.replace(pattern, () => { observed = true; return ''; });
    if (!observed) return definition;
    timingFacts.push({keyword: definition.keyword, timing: 'priority'});
    const bind = value => String(value).replace(ACTION, '$1Any time you have priority, you may pay {Cost} to take a special action.');
    return {...definition, reminder: bind(definition.reminder), expansion: bind(program)};
  });
  return {text: result.split('\n').filter(line => line.trim()).join('\n'), definitions: rewritten, timingFacts};
}
