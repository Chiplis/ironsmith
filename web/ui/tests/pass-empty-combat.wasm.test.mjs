import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import initEngine, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';

test('declaring no attackers skips blockers and damage and reaches second main', async () => {
  await initEngine({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
  const game = new WasmGame();
  try {
    const sources = await Promise.all(['forest', 'akroma-angel-of-wrath'].map(async name =>
      JSON.parse(await readFile(new URL(`../public/cards/${name}.json`, import.meta.url)))));
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(['Alice', 'Bob'], 20);
    game.addCardToZone(0, 'Akroma, Angel of Wrath', 'battlefield', true);
    finishPuzzlePregame(game, { filler: 'Forest' });
    passToFirstMain(game);
    let declared = false, reachedMain = false;
    for (let i = 0; i < 40; i++) {
      const state = game.uiState();
      assert.notEqual(state.decision?.kind, 'blockers');
      assert.doesNotMatch(state.step || '', /blockers|damage/i);
      if (/second.main/i.test(state.phase)) { reachedMain = true; break; }
      if (state.decision?.kind === 'attackers') {
        assert.ok(state.decision.attacker_options.length, 'can choose to attack, but Pass declines');
        game.dispatch({ type: 'declare_attackers', declarations: [] });
        declared = true;
      } else {
        const pass = state.decision?.actions?.find(action => action.action_ref?.kind === 'pass_priority');
        assert.ok(pass, `Expected priority: ${JSON.stringify(state.decision)}`);
        game.dispatch({ type: 'priority_action', action_ref: pass.action_ref });
      }
    }
    assert.ok(declared);
    assert.ok(reachedMain);
  } finally { game.free(); }
});
