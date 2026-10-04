import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import initEngine, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';

test('real WASM retains previous mana availability while timing masks sorceries immediately', async () => {
  await initEngine({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
  const sources = await Promise.all(['mountain', 'lava-spike'].map(async route =>
    JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));
  const game = new WasmGame();
  try {
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(['Alice', 'Bob'], 20);
    game.addCardToZone(0, 'Mountain', 'battlefield', true);
    const spell = Number(game.addCardToZone(0, 'Lava Spike', 'hand', true));
    finishPuzzlePregame(game);
    let state = passToFirstMain(game);
    const hasSpell = state => state.decision?.actions?.some(action => action.kind === 'cast_spell' && Number(action.object_id) === spell);
    assert.ok(hasSpell(state));
    game.rememberPriorityAffordability(state.decision.actions.map(action => action.action_ref));
    game.setDeferredPriorityAnalysis(true);
    game.addCardToZone(0, 'Mountain', 'library', true);
    state = game.uiState();
    assert.equal(state.decision.analysis_complete, false);
    assert.ok(hasSpell(state), 'known affordability remains visible before the new search');
    const finishAnalysis = token => {
      if (!game.beginPriorityAnalysis(token)) return game.uiState().decision;
      for (let i = 0; i < 1000; i++) {
        const decision = game.stepPriorityAnalysis(token, 8);
        assert.notEqual(decision, false);
        if (decision.analysis_complete) return decision;
      }
      throw new Error('analysis did not finish');
    };
    let sawCombat = false;
    for (let i = 0; i < 80; i++) {
      const kind = state.decision?.kind;
      const command = kind === 'priority'
        ? { type: 'priority_action', action_ref: { kind: 'pass_priority' } }
        : ['attackers', 'blockers'].includes(kind)
          ? { type: kind === 'attackers' ? 'declare_attackers' : 'declare_blockers', declarations: [] }
          : null;
      assert.ok(command, `cannot advance fixture: ${kind}`);
      state = game.dispatch(command);
      if (/second.main/i.test(state.phase) && state.priority_player === 0) break;
      if (/combat/i.test(state.phase)) {
        sawCombat = true;
        assert.equal(Boolean(hasSpell(state)), false, 'sorcery timing masks a cached mana positive');
        if (state.decision?.kind === 'priority') {
          const complete = finishAnalysis(`combat:${i}`);
          game.rememberPriorityAffordability(complete.actions.map(action => action.action_ref));
        }
      }
    }
    assert.equal(sawCombat, true);
    assert.match(state.phase, /second.main/i);
    assert.equal(state.decision.analysis_complete, false);
    assert.ok(hasSpell(state), 'combat timing checks do not erase the last mana result');
    const fresh = Number(game.addCardToZone(0, 'Lava Spike', 'hand', true));
    state = game.uiState();
    assert.equal(state.decision.actions.some(action => Number(action.object_id) === fresh), false, 'a new source has no cached positive');
    const complete = finishAnalysis('fresh');
    assert.ok(complete.actions.some(action => Number(action.object_id) === fresh));
  } finally { game.free(); }
});
