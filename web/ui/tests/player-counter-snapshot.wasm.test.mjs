import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import init, { WasmGame } from '../../wasm_demo/pkg/ironsmith.js';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';

await init(Object.fromEntries(await Promise.all(['engine', 'compiler', 'verifier'].map(async name => [
  name, await readFile(new URL(`../../wasm_demo/pkg/${name}_bg.wasm`, import.meta.url)),
]))));

test('JavaScript snapshots expose storm, poison, energy, and experience counters', () => {
  const game = new WasmGame();
  try {
    const sources = [
      { canonicalName: 'Snapshot Counters', group: { kind: 'single', name: 'Snapshot Counters', block: 'Mana Cost: {0}\nType: Sorcery\nYou get two poison counters.\nYou get {E}{E}{E}.\nYou get an experience counter.' } },
      { canonicalName: 'Mountain', group: { kind: 'single', name: 'Mountain', block: 'Type: Basic Land — Mountain' } },
    ];
    assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(sources))).failed, []);
    game.resetEmpty(['Alice', 'Bob'], 20);
    game.addCardToZone(0, 'Snapshot Counters', 'hand', true);
    finishPuzzlePregame(game);
    const before = passToFirstMain(game);
    assert.equal(before.storm_count, 0);
    assert.equal(before.players[0].poison_counters, 0);
    assert.deepEqual(before.players[0].counters, []);
    const cast = before.decision.actions.find(action => action.action_ref?.kind === 'cast_spell');
    assert.ok(cast);
    game.dispatch({ type: 'priority_action', action_ref: cast.action_ref });
    for (let step = 0; step < 10; step++) {
      const state = game.uiState();
      if (!state.stack_size) break;
      const pass = state.decision?.actions?.find(action => action.action_ref?.kind === 'pass_priority');
      assert.ok(pass, JSON.stringify(state.decision));
      game.dispatch({ type: 'priority_action', action_ref: pass.action_ref });
    }
    const after = game.uiState();
    assert.equal(after.stack_size, 0);
    assert.equal(after.storm_count, 1);
    assert.equal(after.players[0].poison_counters, 2);
    assert.deepEqual(Object.fromEntries(after.players[0].counters.map(({ kind, amount }) => [kind, amount])), {
      poison: 2, energy: 3, experience: 1,
    });
  } finally { game.free(); }
});
