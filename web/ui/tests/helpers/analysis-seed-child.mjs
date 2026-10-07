// A fresh process stands in for a new analysis worker: no prior replay state.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { deserialize } from 'node:v8';
import init, { WasmGame, exactSnapshotBuildId, exactSnapshotLayout, replaceEngineInstance, attachExactBuildGame } from '../../../wasm_demo/pkg/engine.js';
import { createExactBuildSnapshotRuntime } from '../../src/lib/exact-build-snapshot.js';
import { createLocalAnalysisReplica } from '../../src/lib/local-analysis-replay.js';
const { replay, expected, publicState } = deserialize(await readFile(process.argv[2]));
const exports = await init({ module_or_path: await readFile(new URL('../../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
const snapshots = createExactBuildSnapshotRuntime({ exports, layout: exactSnapshotLayout, buildId: exactSnapshotBuildId,
  replace: replaceEngineInstance, attach: attachExactBuildGame });
let restores = 0;
const replica = createLocalAnalysisReplica(() => new WasmGame(), {
  restoreSeed: async (image, oldGame) => { restores++; return snapshots.restoreLocal(image, oldGame); },
});
const started = performance.now();
let game = await replica.hydrate(replay);
const hydrateMs = performance.now() - started;
assert.equal(restores, 1);
assert.deepEqual(game.uiState(), expected);
assert.deepEqual(game.exportPublicAuditCheckpoint(), publicState);
// Speculation on the working branch is discarded by the next hydrate.
game.dispatch({ type: 'priority_action', action_ref: { kind: 'pass_priority' } });
game = await replica.hydrate(replay);
assert.equal(restores, 1);
assert.deepEqual(game.uiState(), expected);
console.log(`seeded replica matched (${replay.operations.length} later operations, hydrate ${hydrateMs.toFixed(1)} ms)`);
