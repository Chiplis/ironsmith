import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import initEngine, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';

test('real engine snapshots distinguish first-strike and regular damage priority', async () => {
  await initEngine({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
  const game = new WasmGame();
  try {
    const sources = await Promise.all(['forest', 'akroma-angel-of-wrath'].map(async card =>
      JSON.parse(await readFile(new URL(`../public/cards/${card}.json`, import.meta.url)))));
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(['Alice', 'Bob'], 20);
    const attacker = Number(game.addCardToZone(0, 'Akroma, Angel of Wrath', 'battlefield', true));
    finishPuzzlePregame(game, { filler: 'Forest' });
    passToFirstMain(game);
    const damage = [];
    for (let i = 0; i < 30; i++) {
      const state = game.uiState();
      if (state.decision?.kind === 'priority' && state.combat_damage_step) {
        if (damage.at(-1) !== state.combat_damage_step) damage.push(state.combat_damage_step);
        if (damage.length === 2) break;
      }
      const decision = state.decision;
      if (decision?.kind === 'attackers') {
        game.dispatch({ type: 'declare_attackers', declarations: [{ creature: attacker, target: { kind: 'player', player: 1 } }] });
      } else if (decision?.kind === 'blockers') {
        game.dispatch({ type: 'declare_blockers', declarations: [] });
      } else {
        const pass = decision?.actions?.find(action => action.action_ref?.kind === 'pass_priority');
        assert.ok(pass, `Expected priority: ${JSON.stringify(decision)}`);
        game.dispatch({ type: 'priority_action', action_ref: pass.action_ref });
      }
    }
    assert.deepEqual(damage, ['first_strike', 'regular']);
  } finally { game.free(); }
});
