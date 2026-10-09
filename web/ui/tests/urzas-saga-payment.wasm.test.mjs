import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';

const engineUrl = new URL(process.env.IRONSMITH_TEST_ENGINE || '../../wasm_demo/pkg/engine.js', import.meta.url);
const { default: init, WasmGame } = await import(engineUrl.href);
await init({ module_or_path: await readFile(new URL('./engine_bg.wasm', engineUrl)) });
const sources = await Promise.all(['urza-s-saga', 'mountain'].map(async name =>
  JSON.parse(await readFile(new URL(`../public/cards/${name}.json`, import.meta.url)))));

for (const floating of [false, true]) {
  test(`Saga Construct activation pays with two other lands (${floating ? 'floating' : 'planned'} mana)`, () => {
    const game = new WasmGame();
    try {
      assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(sources))).failed, []);
      game.resetEmpty(['Alice', 'Bob'], 20);
      const saga = Number(game.addCardToZone(0, "Urza's Saga", 'battlefield', true));
      for (let i = 0; i < 2; i++) game.addCardToZone(0, 'Mountain', 'battlefield', true);
      finishPuzzlePregame(game);
      passToFirstMain(game);
      const pass = () => game.dispatch({ type: 'priority_action', action_ref: { kind: 'pass_priority' } });
      for (let i = 0; game.uiState().stack_size && i < 10; i++) pass();
      assert.equal(game.uiState().stack_size, 0);
      if (floating) {
        for (let i = 0; i < 2; i++) {
          const land = game.uiState().decision.actions.find(action => action.kind === 'activate_mana_ability');
          assert.ok(land);
          game.dispatch({ type: 'priority_action', action_ref: land.action_ref });
        }
      }
      const ability = game.uiState().decision.actions.find(action =>
        action.kind === 'activate_ability' && Number(action.object_id) === saga);
      assert.ok(ability, 'chapter II grants a legal Construct activation');
      let state = game.dispatch({ type: 'priority_action', action_ref: ability.action_ref });
      assert.equal(state.decision.kind, 'mana_payment');
      assert.ok(state.mana_payment.can_confirm);
      const { plan_id, request_hash } = state.mana_payment;
      state = game.dispatch({ type: 'mana_payment', response: { action: 'confirm', plan_id, request_hash } });
      assert.equal(state.stack_size, 1, 'confirmation must stack the activation instead of silently rolling back');
      const permanents = state.players[0].battlefield.flatMap(card => card.member_ids || [card.id]);
      assert.ok(permanents.includes(saga));
      assert.ok(state.players[0].battlefield.find(card => Number(card.id) === saga).tapped);
      for (let i = 0; game.uiState().stack_size && i < 10; i++) pass();
      state = game.uiState();
      assert.equal(state.stack_size, 0);
      const construct = state.players[0].battlefield.find(card => card.name === 'Construct');
      assert.ok(construct, 'the Construct survives resolution');
      assert.equal(construct.power_toughness, '1/1');
    } finally { game.free(); }
  });
}
