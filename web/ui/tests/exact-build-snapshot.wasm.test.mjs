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
import { createLocalAnalysisJournal } from '../src/lib/local-analysis-replay.js';
const exports = await init({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
const snapshots = createExactBuildSnapshotRuntime({ exports, layout: exactSnapshotLayout, buildId: exactSnapshotBuildId,
  replace: replaceEngineInstance, attach: attachExactBuildGame });

test('exact-build image survives persistence and a cold process with live effect programs and pending payment', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'ironsmith-exact-snapshot-'));
  const journal = createLocalAnalysisJournal(new WasmGame(), 'persistent-analysis');
  const game = journal.game;
  try {
    const sources = [
      ['Snapshot Untapper', 'Type: Enchantment\nLands you control enter untapped.'],
      ['Snapshot Anthem', 'Type: Enchantment\nCreatures you control get +1/+1.'],
      ['Snapshot Creature', 'Type: Creature — Bear\nPower/Toughness: 2/2'],
      ['Snapshot Tapped Land', 'Type: Land\nThis land enters tapped.\n{T}: Add {R}.'],
      ['Snapshot Mana Land', 'Type: Land\n{T}: Add {R}.'],
      ['Snapshot Spell', 'Mana Cost: {1}\nType: Artifact'],
    ].map(([name, block]) => ({ canonicalName: name, group: { kind: 'single', name, block } }));
    assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(sources))).failed, []);
    game.resetEmpty(['Alice','Bob'], 20);
    for (const name of ['Snapshot Untapper','Snapshot Anthem','Snapshot Mana Land']) game.addCardToZone(0, name, 'battlefield', true);
    game.addCardToZone(0, 'Snapshot Spell', 'hand', true);
    for (const seat of [0,1]) for (let index = 0; index < 8; index++) game.addCardToZone(seat, 'Snapshot Mana Land', 'library', true);
    game.finishPuzzleSetup();
    for (let index = 0; index < 60; index++) {
      const state = game.uiState();
      if (state.active_player === 0 && /first.main/i.test(state.phase)) break;
      const action = state.decision.actions.find(action => ['keep_opening_hand','continue_pregame','begin_game','pass_priority'].includes(action.action_ref?.kind));
      assert.ok(action); game.dispatch({type:'priority_action',action_ref:action.action_ref});
    }
    const action = game.uiState().decision.actions.find(action => action.kind === 'cast_spell' && action.label.includes('Snapshot Spell'));
    assert.ok(action); game.dispatch({type:'priority_action',action_ref:action.action_ref});
    const expected = game.uiState(), publicState = game.exportPublicAuditCheckpoint();
    assert.equal(expected.decision.kind,'mana_payment');
    for (let index=0;index<3;index++) game.createRuntimeSavepoint();
    const image = await snapshots.capture(game, { seq: 12, privateData: new Map([['test', 1n]]), journal: journal.capture() });
    const path = join(directory, 'image');
    await writeFile(path, serialize({ image, expected, publicState }));
    const { stdout } = await promisify(execFile)(process.execPath, [fileURLToPath(new URL('./helpers/exact-snapshot-child.mjs', import.meta.url)), path], { timeout: 60000 });
    assert.match(stdout, /cold restore/);
  } finally { game.free(); await rm(directory,{recursive:true,force:true}); }
});

test('corruption and incompatible builds are rejected before detaching the live game', async () => {
  const game = new WasmGame();
  try {
    game.resetEmpty(['Alice','Bob'],20);
    class HostResource {}
    await assert.rejects(snapshots.capture(game, {host:new HostResource()}), /unsupported host object/);
    const image = await snapshots.capture(game, {}), pointer = game.__wbg_ptr;
    image.memory[image.memory.length - 1] ^= 1;
    await assert.rejects(snapshots.restore(image,game), /integrity mismatch/);
    assert.equal(game.__wbg_ptr,pointer);
    image.buildId = 'other-build';
    await assert.rejects(snapshots.restore(image,game), /incompatible/);
    assert.equal(game.uiState().players[0].life,20);
  } finally { game.free(); }
});
