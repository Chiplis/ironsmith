import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import initEngine, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';
import { hasPotentialAttackers } from '../src/lib/priority-stops.js';

const pass = { type: 'priority_action', action_ref: { kind: 'pass_priority' } };

test('real WASM answers I can act stops: mana abilities and sorceries off-turn never count', async () => {
  await initEngine({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
  const sources = await Promise.all(['mountain', 'lava-spike', 'shock', 'grizzly-bears'].map(async route =>
    JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));
  const game = new WasmGame();
  try {
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(['Alice', 'Bob'], 20);
    game.addCardToZone(0, 'Mountain', 'battlefield', true);
    game.addCardToZone(0, 'Grizzly Bears', 'battlefield', true);
    game.addCardToZone(0, 'Lava Spike', 'hand', true);
    finishPuzzlePregame(game);
    let state = passToFirstMain(game);
    assert.equal(state.decision.can_act, true, 'a castable sorcery in your main phase');
    assert.equal(hasPotentialAttackers(state), false, 'the Bears arrived this turn');
    for (let i = 0; i < 100; i++) {
      state = game.uiState();
      if (state.active_player === 0 && state.turn_number >= 3 && /first.main/i.test(state.phase)) break;
      game.dispatch(state.decision?.kind === 'attackers' ? { type: 'declare_attackers', declarations: [] } : pass);
    }
    assert.equal(hasPotentialAttackers(state), true, 'next turn the Bears can attack');

    // Bob's turn: Alice holds only a sorcery, so her stops stay quiet without
    // waiting for the payment search, even in the browser's deferred mode.
    game.setDeferredPriorityAnalysis(true);
    for (let i = 0; i < 60; i++) {
      state = game.uiState();
      if (state.active_player === 1 && state.decision?.kind === 'priority' && state.decision.player === 0) break;
      if (state.decision?.kind === 'attackers') game.dispatch({ type: 'declare_attackers', declarations: [] });
      else game.dispatch(pass);
    }
    assert.equal(state.active_player, 1);
    assert.equal(state.decision.can_act, false, 'the seeded menu already knows nothing is eligible');
    assert.equal(game.getPriorityCanAct(), false);

    // An instant is eligible: unknown until its payment is proven, then yes.
    game.addCardToZone(0, 'Shock', 'hand', true);
    state = game.uiState();
    assert.equal(state.decision.can_act ?? null, null, 'waits for the payment search');
    assert.ok(game.beginPriorityAnalysis('can-act'));
    let decision = null;
    for (let i = 0; i < 1000 && !decision?.analysis_complete; i++) decision = game.stepPriorityAnalysis('can-act', 8);
    assert.equal(decision.can_act, true);
  } finally {
    game.free();
  }
});
