// Independent readers for the instructions attached to a layout's ability
// bars. The declaration name supplies identity; it never selects behavior.
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

export function readAbilityBarProgram(reminder) {
  const sentences = String(reminder).split('.').map(sentence => sentence.trim().toLowerCase()).filter(Boolean);
  if (!sentences.includes('ability bars are activated abilities')) return null;
  const recognized = new Set([
    'ability bars are activated abilities', 'each bar costs its printed cost',
    'each bar adds its printed subtype', 'each bar puts a +1/+1 counter on this permanent',
    'each bar performs its printed effect', 'activate bars only once',
    'activate bars only as a sorcery', 'activate bars in printed order',
    'activate a bar only if no bar on this permanent has been activated',
  ]);
  if (sentences.some(sentence => !recognized.has(sentence))) throw Error('Unrecognized ability-bar instruction.');
  if (!sentences.includes('each bar costs its printed cost') || !sentences.includes('each bar performs its printed effect'))
    throw Error('An ability-bar program must preserve its printed cost and effect.');
  const ordered = sentences.includes('activate bars in printed order');
  const exclusive = sentences.includes('activate a bar only if no bar on this permanent has been activated');
  if (ordered && exclusive) throw Error('A group cannot be both ordered and exclusive.');
  if (!ordered && !exclusive && !sentences.includes('activate bars only once')) throw Error('An ability-bar group needs its activation-history rule.');
  return { policy: ordered ? 'Sequential' : exclusive ? 'Exclusive' : 'Once',
    subtype: sentences.includes('each bar adds its printed subtype'),
    counter: sentences.includes('each bar puts a +1/+1 counter on this permanent'),
    sorcery: sentences.includes('activate bars only as a sorcery') };
}

export function expandAbilityBarProgram(text, definition) {
  const program = readAbilityBarProgram(definition.reminder);
  if (!program) return text;
  const declaration = new RegExp(`^\\s*${escape(definition.keyword)}\\s*$`, 'i');
  const lines = String(text).split('\n');
  let active = false, group = -1, step = 0;
  // Named ability families stay separate even when a permanent gains groups
  // using the same history policy. FNV-1a is deterministic across imports.
  let family = 2166136261;
  for (const code of String(definition.keyword).normalize('NFKC').toLowerCase()) {
    family = Math.imul(family ^ code.codePointAt(0), 16777619) >>> 0;
  }
  const result = [];
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index];
    if (declaration.test(line)) { active = true; group++; step = 0; continue; }
    // A bar is a structural cost/label delimiter, not any ordinary activated
    // ability in the top box. Prefix lines and unrelated prose are retained.
    const barPattern = /^\s*(.+?):\s*([\p{L}][\p{L} '\-]*?)(?:\s*[—–]\s*(.+))?$/u;
    const bar = active && line.match(barPattern);
    if (!bar) { result.push(line); continue; }
    const [, cost, label, inlineEffect] = bar;
    const effectLines = inlineEffect ? [inlineEffect] : [];
    while (index + 1 < lines.length && !barPattern.test(lines[index + 1]) && !declaration.test(lines[index + 1])) {
      effectLines.push(lines[++index]);
    }
    const effect = effectLines.join(' ').trim();
    if (!effect) throw Error('An ability bar must retain its complete printed effect.');
    const body = [];
    if (program.subtype) body.push(`This permanent becomes a ${label.trim()} in addition to its other types.`);
    if (program.counter) body.push('Put a +1/+1 counter on this permanent.');
    body.push(effect.trim().replace(/[.!?]*$/, '.'));
    if (program.sorcery) body.push('Activate only as a sorcery.');
    result.push(`Family ${family} ${program.policy} group ${group} step ${step++} — ${cost.trim()}: ${body.join(' ')}`);
  }
  if (active && step === 0) throw Error('An ability-bar declaration needs complete cost, label, and effect bars.');
  return result.join('\n');
}
