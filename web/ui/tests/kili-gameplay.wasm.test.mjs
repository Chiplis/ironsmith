import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { finishPuzzlePregame, passToFirstMain } from './fixtures/native-game-setup.mjs';

const engineUrl = new URL(process.env.IRONSMITH_TEST_ENGINE || '../../wasm_demo/pkg/engine.js', import.meta.url);
const { default: init, WasmGame } = await import(engineUrl.href);
await init({ module_or_path: await readFile(new URL('./engine_bg.wasm', engineUrl)) });
const sources = await Promise.all(['kili-the-resourceful', 'short-sword', 'ornithopter', 'dwarven-trader', 'mountain'].map(async route =>
  JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));
const KILI = 'Kíli the Resourceful';

test('Kíli’s bundled artifact retains the story gate and once-per-turn draw text', () => {
  const artifact = sources[0].artifacts[0];
  assert.equal(artifact.diagnostics.errorCount, 0);
  assert.equal(artifact.payload.canonicalText, [
    'Storied.',
    'As long as you have an enduring story, you may pay {0} rather than pay the equip cost of the first equip ability you activate each turn.',
    'Whenever another Dwarf or Equipment you control enters, draw a card. This ability triggers only once each turn.',
  ].join('\n'));
});

function setup(names = [KILI]) {
  const game = new WasmGame();
  assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(sources))).failed, []);
  game.resetEmpty(['Alice', 'Bob'], 20);
  for (const name of names) game.addCardToZone(0, name, 'battlefield', true);
  finishPuzzlePregame(game);
  passToFirstMain(game);
  return game;
}

function settle(game) {
  for (let step = 0; step < 20; step++) {
    const state = game.uiState();
    if (!state.stack_size) return state;
    assert.equal(state.decision.kind, 'priority', JSON.stringify(state.decision));
    game.dispatch({ type: 'priority_action', action_ref: { kind: 'pass_priority' } });
  }
  assert.fail('Stack did not settle');
}

function advanceToMain(game, player, turn) {
  for (let step = 0; step < 100; step++) {
    const state = game.uiState();
    if (state.active_player === player && state.turn_number >= turn && /first.main/i.test(state.phase)) return;
    const decision = state.decision;
    if (decision.kind === 'attackers') {
      game.dispatch({ type: 'declare_attackers', declarations: [] });
    } else {
      assert.equal(decision.kind, 'priority', JSON.stringify(decision));
      game.dispatch({ type: 'priority_action', action_ref: { kind: 'pass_priority' } });
    }
  }
  assert.fail('Did not reach the requested main phase');
}

test('Kíli draws for another Dwarf or Equipment, once each turn, including opponents’ turns', () => {
  for (const first of ['Dwarven Trader', 'Short Sword']) {
    const game = setup();
    try {
      const before = game.uiState().players[0];
      game.addCardToZone(0, first, 'battlefield', false);
      assert.equal(game.uiState().stack_size, 1, `${first} creates a draw trigger`);
      // A second qualifying entry before resolution must not queue another draw.
      game.addCardToZone(0, first === 'Short Sword' ? 'Dwarven Trader' : 'Short Sword', 'battlefield', false);
      assert.equal(game.uiState().stack_size, 1, 'the limit applies at triggering, not resolution');
      let state = settle(game);
      assert.equal(state.players[0].hand_size, before.hand_size + 1);
      assert.equal(state.players[0].library_size, before.library_size - 1);
      game.addCardToZone(0, 'Short Sword', 'battlefield', false);
      assert.equal(settle(game).players[0].hand_size, before.hand_size + 1);
      advanceToMain(game, 1, 2);
      const next = game.uiState().players[0].hand_size;
      game.addCardToZone(0, 'Dwarven Trader', 'battlefield', false);
      assert.equal(settle(game).players[0].hand_size, next + 1, 'limit resets on the opponent’s turn');
    } finally { game.free(); }
  }
});

test('Kíli excludes himself, unrelated artifacts and opposing Dwarves/Equipment', () => {
  const game = setup([]);
  try {
    const before = game.uiState().players.map(player => player.hand_size);
    for (const [seat, name] of [[0, KILI], [0, 'Ornithopter'], [1, 'Dwarven Trader'], [1, 'Short Sword']]) {
      game.addCardToZone(seat, name, 'battlefield', false);
      assert.equal(game.uiState().stack_size, 0, `${seat}: ${name} must not trigger`);
      assert.deepEqual(game.uiState().players.map(player => player.hand_size), before);
    }
    game.addCardToZone(0, 'Short Sword', 'battlefield', false);
    assert.equal(settle(game).players[0].hand_size, before[0] + 1, 'excluded entries do not consume the trigger');
  } finally { game.free(); }
});

test('Kíli shows Equip {0}, skips empty payment, and consumes the first equip allowance', () => {
  for (const enduring of [false, true]) {
    const game = setup([KILI, 'Short Sword', ...(enduring ? ['Ornithopter'] : [])]);
    try {
      let state = game.uiState();
      const kili = state.players[0].battlefield.find(card => card.name === KILI);
      const sword = state.players[0].battlefield.find(card => card.name === 'Short Sword');
      const equip = game.inspectorActions(BigInt(sword.id)).find(action => action.kind === 'activate_ability');
      assert.match(equip.label, enduring ? /Equip \{0\}/ : /Equip \{1\}/);
      assert.equal(equip.mana_payment_available, enduring);
      if (!enduring) game.addCardToZone(0, 'Mountain', 'battlefield', true);
      state = game.dispatch({ type: 'priority_action', action_ref: equip.action_ref });
      assert.equal(state.decision.kind, 'targets');
      state = game.dispatch({ type: 'select_targets', targets: [{ kind: 'object', object: kili.id }] });
      if (!enduring) {
        assert.equal(state.decision.kind, 'mana_payment', 'a nonzero cost still requires payment');
        assert.deepEqual(state.mana_payment.pips, [['1']]);
        state = game.dispatch({ type: 'mana_payment', response: {
          action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash,
        } });
      }
      assert.equal(state.decision.kind, 'priority', 'zero cost must not open a payment confirmation');
      assert.equal(state.stack_size, 1);
      state = settle(game);
      assert.equal(state.players[0].battlefield.find(card => card.id === kili.id).power_toughness, '2/3');
      assert.ok(Object.values(state.players[0].mana_pool).every(value => value === 0));
      const second = game.inspectorActions(BigInt(sword.id)).find(action => action.kind === 'activate_ability');
      assert.match(second.label, /Equip \{1\}/);
      // The planner may omit its availability estimate when no mana source exists.
      assert.notEqual(second.mana_payment_available, true);
      if (!enduring) {
        game.addCardToZone(0, 'Ornithopter', 'battlefield', true);
        const afterStory = game.inspectorActions(BigInt(sword.id)).find(action => action.kind === 'activate_ability');
        assert.match(afterStory.label, /Equip \{1\}/, 'gaining a story does not reset the first-equip allowance');
      }
      advanceToMain(game, 1, 2);
      advanceToMain(game, 0, 3);
      const nextTurn = game.inspectorActions(BigInt(sword.id)).find(action => action.kind === 'activate_ability');
      assert.match(nextTurn.label, /Equip \{0\}/);
      assert.equal(nextTurn.mana_payment_available, true);
    } finally { game.free(); }
  }
});
