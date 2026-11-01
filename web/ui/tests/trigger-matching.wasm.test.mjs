import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createPriorityPolicy } from '../src/lib/priority-policy.js';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';

const engineUrl = new URL(process.env.IRONSMITH_TEST_ENGINE || '../../wasm_demo/pkg/engine.js', import.meta.url);
const { default: initEngine, WasmGame } = await import(engineUrl.href);

test('matching Mystics automatically order on successive casts without merging their triggers', async () => {
  await initEngine({ module_or_path: await readFile(new URL('./engine_bg.wasm', engineUrl)) });
  const sources = await Promise.all(['mountain', 'murmuring-mystic', 'rite-of-flame'].map(async card =>
    JSON.parse(await readFile(new URL(`../public/cards/${card}.json`, import.meta.url)))));
  const game = new WasmGame();
  try {
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    game.resetEmpty(['Alice', 'Bob'], 20);
    for (let i = 0; i < 2; i++) {
      game.addCardToZone(0, 'Murmuring Mystic', 'battlefield', true);
      game.addCardToZone(0, 'Mountain', 'battlefield', true);
      game.addCardToZone(0, 'Rite of Flame', 'hand', true);
    }
    finishPuzzlePregame(game);
    let state = passToFirstMain(game);
    const policy = createPriorityPolicy();
    for (let cast = 0; cast < 2; cast++) {
      const action = state.decision.actions.find(action => action.kind === 'cast_spell');
      assert.ok(action);
      state = game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
      assert.equal(state.decision.kind, 'mana_payment');
      state = game.dispatch({ type: 'mana_payment', response: {
        action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash,
      } });
      assert.equal(state.decision.choice_kind, 'trigger_order');
      assert.equal(state.decision.options.length, 2);
      if (cast === 0) {
        assert.equal(policy.evaluate(state), null, 'ordering stays manual until enabled');
        policy.setGroupTriggers(true, state);
      }
      const command = policy.evaluate(state);
      assert.deepEqual(command, { type: 'select_options', option_indices: [0, 1] },
        'equivalent abilities on separate sources match, including on later casts');
      state = game.dispatch(command);
      assert.equal(state.stack_size, 3, 'both triggers and the spell remain on the stack');
      const mystics = state.stack_objects.filter(entry => entry.name === 'Murmuring Mystic');
      assert.equal(mystics.length, 2);
      assert.notEqual(mystics[0].automation_identity, mystics[1].automation_identity,
        'source-specific runtime identities remain distinct');
      for (let i = 0; i < 12 && (state.stack_size || state.decision.player !== 0); i++) {
        assert.equal(state.decision.kind, 'priority');
        state = game.dispatch({ type: 'priority_action', action_ref: { kind: 'pass_priority' } });
      }
      assert.equal(state.stack_size, 0);
      assert.equal(state.players[0].battlefield.reduce((total, card) => total + (card.count || 1), 0),
        4 + 2 * (cast + 1), 'each Mystic creates its own token on each cast');
    }
  } finally { game.free(); }
});
