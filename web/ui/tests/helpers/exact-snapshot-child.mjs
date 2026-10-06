// A separate process has no old instance, function references or JS caches.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { deserialize } from 'node:v8';
import init, { WasmGame, exactSnapshotBuildId, exactSnapshotLayout, replaceEngineInstance, attachExactBuildGame } from '../../../wasm_demo/pkg/engine.js';
import { createExactBuildSnapshotRuntime } from '../../src/lib/exact-build-snapshot.js';
import { createLocalAnalysisJournal, createLocalAnalysisReplica, releaseRestoredRuntimeSavepoints } from '../../src/lib/local-analysis-replay.js';
const { image, expected, publicState } = deserialize(await readFile(process.argv[2]));
const exports = await init({ module_or_path: await readFile(new URL('../../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
const snapshots = createExactBuildSnapshotRuntime({ exports, layout: exactSnapshotLayout, buildId: exactSnapshotBuildId,
  replace: replaceEngineInstance, attach: attachExactBuildGame });
let game = new WasmGame();
game = await snapshots.restore(image, game);
const journal = createLocalAnalysisJournal(game, 'restored-analysis', image.recovery.journal);
game = journal.game;
releaseRestoredRuntimeSavepoints(journal);
const freshHandles=Array.from({length:16},()=>game.createRuntimeSavepoint());
for(const handle of freshHandles)game.releaseRuntimeSavepoint(handle);
assert.deepEqual(game.uiState(), expected);
assert.deepEqual(game.exportPublicAuditCheckpoint(), publicState);
const replica = createLocalAnalysisReplica(() => new WasmGame());
const analysis = await replica.hydrate(journal.capture());
assert.deepEqual(analysis.uiState().decision, expected.decision, 'auxiliary analysis resumes from the restored local journal');
analysis.free();
const payment = game.uiState().mana_payment;
assert.equal(game.dispatch({ type: 'mana_payment', response: { action: 'confirm', plan_id: payment.plan_id, request_hash: payment.request_hash } }).decision.kind, 'priority');
// Pending payment and its live executable continuation survive a cold instance.
assert.ok(game.uiState().stack_objects.some(object => object.name === 'Snapshot Spell'));
// Static replacement matcher/program and dynamic executable card definitions
// remain callable through the recreated function table.
game.addCardToZone(0, 'Snapshot Tapped Land', 'battlefield', false);
let decision = game.uiState().decision;
assert.equal(decision.reason, 'Replacement effect');
for (let index = 0; index < 4 && decision.reason === 'Replacement effect'; index++) {
  const option = decision.options.find(option => option.description === 'Snapshot Tapped Land') || decision.options[0];
  game.dispatch({ type: 'select_options', option_indices: [option.index] });
  decision = game.uiState().decision;
}
assert.equal(game.exportPublicAuditCheckpoint().objects.find(object => object.identity?.name === 'Snapshot Tapped Land').tapped, false);
// Retained continuous effect programs still calculate characteristics.
const creature = game.addCardToZone(0, 'Snapshot Creature', 'battlefield', true);
assert.equal(game.uiState().players[0].battlefield.find(object => String(object.id) === String(creature)).power_toughness, '3/3');
game.free();
console.log('cold restore: pending payment, replacement, continuous effect, and destruction passed');
