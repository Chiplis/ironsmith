// Structural probes only: a passing result is never a gameplay-equivalence claim.
import { readFileSync, appendFileSync, existsSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { probeMechanic, validateMechanic } from '../../web/ui/src/lib/custom-mechanics.js';
import { NativeWorker } from './native-worker-client.mjs';
const [source, output, executable, count = '3'] = process.argv.slice(2);
if (!source || !output || !executable) throw Error('usage: node probe-definitions.mjs SOURCE.json OUTPUT.jsonl NATIVE_WORKER [WORKERS]');
const definitions = JSON.parse(readFileSync(source));
process.env.AUDIT_NATIVE_WORKER = resolve(executable);
const digest = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const provenance = { source: resolve(source), source_sha256: digest(source), executable: resolve(executable), executable_sha256: digest(executable), filters_enforced: true, lines_enforced: true, allow_unsupported: false };
const provenancePath = output + '.provenance.json';
if (existsSync(provenancePath) && JSON.stringify(JSON.parse(readFileSync(provenancePath))) !== JSON.stringify(provenance)) throw Error('Refusing to mix results from different inputs or executables');
writeFileSync(provenancePath, JSON.stringify(provenance, null, 2));
const done = new Set(existsSync(output) ? readFileSync(output, 'utf8').trim().split('\n').filter(Boolean).map(line => JSON.parse(line).index) : []);
function compileInput(draft) {
  return { ...draft, faces: draft.faces.map(f => ({ ...f, parse_input: [f.manaCost ? `Mana cost: ${f.manaCost}` : '', `Type: ${f.typeLine || [...(f.supertypes || []), ...(f.cardTypes || [])].join(' ')}`, f.power !== '' && f.power != null ? `Power/Toughness: ${f.power}/${f.toughness}` : '', f.loyalty ? `Loyalty: ${f.loyalty}` : '', f.defense ? `Defense: ${f.defense}` : '', f.oracleText].filter(Boolean).join('\n') })) };
}
let cursor = 0;
async function consume() {
  const worker = new NativeWorker();
  try {
    while (cursor < definitions.length) {
      const index = cursor++;
      if (done.has(index)) continue;
      const source = definitions[index];
      const row = { index, format: source.format, keyword: source.keyword, reminder: source.reminder, source_cards: source.source_cards, attempts: [], semantic_gate: 'requires_behavioral_verification' };
      try {
        row.definition = validateMechanic(source);
        row.result = await probeMechanic(source, async draft => {
          const input = compileInput(draft);
          const result = await worker.run(input);
          row.attempts.push({ input, result });
          if (!result.canCreate) throw Error(result.faces.map(f => f.error || f.loss_diagnostics || '').join('\n') || 'compile failed');
          return result;
        });
      } catch (error) { row.error = String(error?.message || error); }
      appendFileSync(output, JSON.stringify(row) + '\n');
      done.add(index);
      if (done.size % 25 === 0) console.log(`probed ${done.size}/${definitions.length}`);
    }
  } finally { worker.stop(); }
}
await Promise.all(Array.from({ length: Math.max(1, Number(count)) }, consume));
console.log(`completed ${done.size}/${definitions.length}`);
