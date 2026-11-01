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

// Exercise the same policy commands used by GameContext against real engine
// snapshots, rather than manually issuing the empty attack declaration.
test('combat policy defaults to choosing legal attackers, then skips them only with icon off and no combat stops', async () => {
  const { createPriorityPolicy } = await import('../src/lib/priority-policy.js');
  const { createPriorityStops } = await import('../src/lib/priority-stops.js');
  await initEngine({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
  const sources = await Promise.all(['forest', 'akroma-angel-of-wrath'].map(async name =>
    JSON.parse(await readFile(new URL(`../public/cards/${name}.json`, import.meta.url)))));
  for (const scenario of ['default', 'off', 'off-with-damage-stop']) {
    const game = new WasmGame();
    try {
      game.registerExternalCardSourcesJson(JSON.stringify(sources));
      game.resetEmpty(['Alice', 'Bob'], 20);
      game.addCardToZone(0, 'Akroma, Angel of Wrath', 'battlefield', true);
      finishPuzzlePregame(game, { filler: 'Forest' });
      passToFirstMain(game);
      const stops = createPriorityStops({ initialStops: { 'phase:Main': 'off' } });
      const policy = createPriorityPolicy({ stops, storage: null });
      let state = game.uiState();
      assert.equal(policy.status(state).stopWithAttackers, true, 'attacker icon starts enabled');
      assert.equal(policy.status(state).stopWithBlockers, true, 'blocker icon starts enabled');
      if (scenario !== 'default') policy.setCombatParticipationStop('attackers', false, { ...state, perspective: 0 });
      if (scenario === 'off-with-damage-stop') stops.setStop('step:CombatDamage', 'always', state, 0);
      let declared = false, stoppedAtAttackers = false, reachedMain = false;
      for (let i = 0; i < 40; i++) {
        state = game.uiState();
        const view = { ...state, perspective: state.decision?.player };
        if (/second.main/i.test(state.phase)) { reachedMain = true; break; }
        assert.notEqual(state.decision?.kind, 'blockers');
        const command = policy.evaluate(view);
        if (state.decision?.kind === 'attackers') {
          assert.ok(state.decision.attacker_options.some(option => option.valid_targets.length > 0), 'real legal attacker exists');
          if (scenario === 'off') {
            assert.deepEqual(command, { type: 'declare_attackers', declarations: [] });
            declared = true;
          } else {
            assert.equal(command, null, 'default icon or explicit combat stop preserves the attack choice');
            stoppedAtAttackers = true;
            break;
          }
        }
        assert.ok(command, `Unexpected stop in ${scenario}: ${JSON.stringify(state.decision)}`);
        stops.beforeCommand(command, view);
        policy.beforeAutomaticCommand(command, view);
        game.dispatch(command);
      }
      assert.equal(declared, scenario === 'off');
      assert.equal(reachedMain, scenario === 'off', 'automatic empty declaration advances to second main');
      assert.equal(stoppedAtAttackers, scenario !== 'off');
    } finally { game.free(); }
  }
});
