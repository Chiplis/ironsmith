import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { serialize } from 'node:v8';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import init, { WasmGame, exactSnapshotBuildId, exactSnapshotLayout, replaceEngineInstance, attachExactBuildGame } from '../../wasm_demo/pkg/engine.js';
import { createExactBuildSnapshotRuntime } from '../src/lib/exact-build-snapshot.js';
import { createLocalAnalysisJournal, localReplayEnd, seededLocalReplay } from '../src/lib/local-analysis-replay.js';
const exports = await init({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
const snapshots = createExactBuildSnapshotRuntime({ exports, layout: exactSnapshotLayout, buildId: exactSnapshotBuildId,
  replace: replaceEngineInstance, attach: attachExactBuildGame });

function advance(game, steps) {
  for (let index = 0; index < steps; index++) {
    const state = game.uiState();
    const action = state.decision?.actions?.find(action => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(action.action_ref?.kind));
    assert.ok(action, `cannot advance from ${state.decision?.kind}`);
    game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
  }
}

test('an analysis replica seeded from a local image matches the session and replays only later calls', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'ironsmith-analysis-seed-'));
  const journal = createLocalAnalysisJournal(new WasmGame(), 'seeded-analysis');
  const game = journal.game;
  try {
    const sources = [
      ['Seed Anthem', 'Type: Enchantment\nCreatures you control get +1/+1.'],
      ['Seed Creature', 'Type: Creature — Bear\nPower/Toughness: 2/2'],
      ['Seed Land', 'Type: Land\n{T}: Add {R}.'],
    ].map(([name, block]) => ({ canonicalName: name, group: { kind: 'single', name, block } }));
    assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(sources))).failed, []);
    game.resetEmpty(['Alice', 'Bob'], 20);
    game.addCardToZone(0, 'Seed Anthem', 'battlefield', true);
    for (const seat of [0, 1]) for (let index = 0; index < 12; index++) game.addCardToZone(seat, 'Seed Land', 'library', true);
    game.finishPuzzleSetup();
    advance(game, 6);
    // A branch alive at seed time keeps its handle inside the image.
    const branch = game.createRuntimeSavepoint();
    const seeded = seededLocalReplay(journal.capture(), snapshots.captureLocal(game));
    assert.ok(seeded.base > 0);
    advance(game, 4);
    game.addCardToZone(0, 'Seed Creature', 'battlefield', true);
    game.exchangeRuntimeSavepoint(branch);
    game.exchangeRuntimeSavepoint(branch);
    game.releaseRuntimeSavepoint(branch);
    const full = journal.capture();
    const replay = { ...seeded, operations: full.operations.slice(seeded.base) };
    assert.equal(localReplayEnd(replay), full.operations.length);
    const expected = game.uiState(), publicState = game.exportPublicAuditCheckpoint();
    assert.equal(expected.players[0].battlefield.find(object => object.name === 'Seed Creature').power_toughness, '3/3');
    const path = join(directory, 'seed');
    await writeFile(path, serialize({ replay, expected, publicState }));
    const { stdout } = await promisify(execFile)(process.execPath,
      [fileURLToPath(new URL('./helpers/analysis-seed-child.mjs', import.meta.url)), path], { timeout: 60000 });
    assert.match(stdout, /seeded replica matched/);
  } finally { game.free(); await rm(directory, { recursive: true, force: true }); }
});
