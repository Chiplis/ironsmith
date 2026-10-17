import { readFile, writeFile } from 'node:fs/promises';
import { performance } from 'node:perf_hooks';
import inspector from 'node:inspector';
import init, { WasmGame } from '../web/wasm_demo/pkg/ironsmith.js';

// Reproduce a local Diagnostics-sheet export against the currently built WASM.
// Run benchmarks sequentially: concurrent CPU work invalidates timing comparisons.
// GOLDFISH_RUNS defaults to 3; run 1 captures a V8 CPU profile.
// The optional card/ability ablations apply only to this private replay.
const reportPath = process.argv[2];
const outputPath = process.argv[3];
if (!reportPath || !outputPath) throw new Error('Usage: node scripts/benchmark-goldfish-journal.mjs <diagnostics.json> <results.json>');
const report = JSON.parse(await readFile(reportPath, 'utf8'));
if (!report.journal?.replayable || report.journal.policy !== 'full') throw new Error('A full, replayable local diagnostics journal is required');
const removed = new Set((process.env.GOLDFISH_REPLACE_CARDS || '').split('|').filter(Boolean));
await init({ engine: await readFile(process.env.GOLDFISH_ENGINE_WASM || new URL('../web/wasm_demo/pkg/engine_bg.wasm', import.meta.url)), compiler: false, verifier: false });
const session = new inspector.Session();
session.connect();
const post = (method, args = {}) => new Promise((resolve, reject) => session.post(method, args, (error, result) => error ? reject(error) : resolve(result)));
const results = [];
for (let run = 0; run < Number(process.env.GOLDFISH_RUNS || 3); run++) {
  const game = new WasmGame();
  game.setDeferredPriorityAnalysis(true);
  for (const route of report.journal.cardRoutes) {
    const source = JSON.parse(await readFile(new URL(`../web/ui/public/cards/${route}.json`, import.meta.url), 'utf8'));
    if (route === 'the-golden-throne' && process.env.GOLDFISH_THRONE_ABILITY) {
      delete source.artifacts;
      source.group.block = source.group.block.split('\n').filter(line =>
        process.env.GOLDFISH_THRONE_ABILITY === 'mana' ? !line.startsWith('Arcane Life-support') : !line.startsWith('A Thousand Souls')).join('\n');
    }
    const registered = game.registerExternalCardSources(source);
    if (registered.failed?.length) throw new Error(JSON.stringify(registered.failed));
  }
  if (run === 1) { await post('Profiler.enable'); await post('Profiler.setSamplingInterval', { interval: 1000 }); await post('Profiler.start'); }
  for (const entry of report.journal.entries) {
    const args = structuredClone(entry.args);
    if (entry.method === 'addCardsToZones') {
      for (const card of args[0]) {
        if (removed.has(card.cardName) || (process.env.GOLDFISH_BASIC_LIBRARIES && card.zoneName === 'library')) card.cardName = 'Plains';
      }
    }
    const before = game.uiState();
    const started = performance.now();
    const result = game[entry.method](...args);
    const elapsedMs = performance.now() - started;
    const dispatch = entry.method === 'dispatch' ? game.lastDispatchPerf() : null;
    results.push({ run, seq: entry.seq, method: entry.method, command: entry.method === 'dispatch' ? entry.args[0] : null,
      before: { phase: before.phase, step: before.step, player: before.priority_player }, elapsedMs,
      dispatch, snapshot: game.lastSnapshotPerf(), after: result?.decision?.kind });
  }
  if (run === 1) { const { profile } = await post('Profiler.stop'); await writeFile(`${outputPath}.cpuprofile`, JSON.stringify(profile)); }
  game.free();
}
session.disconnect();
await writeFile(outputPath, JSON.stringify(results, null, 2));
console.log(JSON.stringify(results.filter(row => row.command?.action_ref?.kind === 'pass_priority').map(row => ({ run: row.run, seq: row.seq, ...row.before, elapsedMs: +row.elapsedMs.toFixed(2), commandToResponseMs: row.dispatch?.commandToResponseMs, snapshotMs: row.snapshot?.totalSnapshotMs })), null, 2));
