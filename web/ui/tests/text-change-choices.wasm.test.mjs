import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import initEngine, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';

test('Artificial Evolution asks for the replacement creature type after the original', async () => {
  await initEngine({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
  const sources = await Promise.all(['artificial-evolution', 'island', 'nightmare', 'swamp'].map(async route =>
    JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));
  const game = new WasmGame();
  try {
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(['Alice', 'Bob'], 20);
    game.addCardToZone(0, 'Artificial Evolution', 'hand', true);
    game.addCardToZone(0, 'Island', 'battlefield', true);
    game.addCardToZone(0, 'Swamp', 'battlefield', true);
    const target = game.addCardToZone(0, 'Nightmare', 'battlefield', true);
    finishPuzzlePregame(game, { filler: 'Island' });
    passToFirstMain(game);
    let state = game.uiState();
    const cast = state.decision.actions.find(action => action.kind === 'cast_spell' && action.label.includes('Artificial Evolution'));
    assert.ok(cast);
    state = game.dispatch({ type: 'priority_action', action_ref: cast.action_ref });
    state = game.dispatch({ type: 'select_targets', targets: [{ kind: 'object', object: Number(target) }] });
    assert.equal(state.decision.kind, 'mana_payment');
    state = game.dispatch({ type: 'mana_payment', response: { action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash } });
    for (let step = 0; step < 10 && state.decision.kind === 'priority'; step++) {
      const pass = state.decision.actions.find(action => action.action_ref?.kind === 'pass_priority');
      state = game.dispatch({ type: 'priority_action', action_ref: pass.action_ref });
    }
    assert.equal(state.decision.description, 'Choose the word to replace');
    const original = state.decision.options.find(option => option.description === 'Nightmare');
    state = game.dispatch({ type: 'select_options', option_indices: [original.index] });
    assert.equal(state.decision.description, 'Choose the replacement word');
    assert.ok(!state.decision.options.some(option => ['Wall', 'Nightmare'].includes(option.description)));
    const human = state.decision.options.find(option => option.description === 'Human');
    state = game.dispatch({ type: 'select_options', option_indices: [human.index] });
    assert.equal(state.stack_size, 0);
    assert.match(game.objectDetails(target).type_line, /Human/);
  } finally { game.free(); }
});
