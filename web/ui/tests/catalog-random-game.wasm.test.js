import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import init, { WasmGame } from '../../wasm_demo/pkg/ironsmith.js';
import { buildCatalogRandomGame } from '../src/lib/catalog-random-game.js';
import { buildOpponentDecisionCommand } from '../src/lib/opponent-decision.js';
import { createSeededRng } from '../src/lib/random-game.js';
const fetchImpl = async url => {
  try {
    const contents = await readFile(new URL(`../public${new URL(url).pathname}`, import.meta.url), 'utf8');
    return { ok: true, json: async () => JSON.parse(contents) };
  } catch { return { ok: false, status: 404 }; }
};
const modules = await Promise.all(['engine', 'compiler', 'verifier'].map(async name => [name,
  await readFile(new URL(`../../wasm_demo/pkg/${name}_bg.wasm`, import.meta.url))]));
await init(Object.fromEntries(modules));
async function register(game, names) {
  const index = await (await fetchImpl('http://localhost/cards/index.json')).json();
  assert.ok(Array.isArray(index.cards), JSON.stringify({ keys: Object.keys(index), index }));
  const routes = new Map(index.cards.map(card => [card.name, card.route]));
  for (const name of new Set(names)) {
    const source = await (await fetchImpl(`http://localhost/cards/${routes.get(name)}.json`)).json();
    assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(source))).failed, []);
  }
}

test('real engine accepts the complete catalog 1v1 position', { timeout: 240000 }, async () => {
  const payload = await buildCatalogRandomGame({ fetchImpl, rng: createSeededRng('engine-catalog'), minScore: .96 });
  const game = new WasmGame();
  try {
    await register(game, payload.players.flatMap(player => Object.values(player.zones).flat()));
    game.resetEmpty(['Alice', 'Bob'], 20);
    const placements = payload.players.flatMap((player, playerIndex) => Object.entries(player.zones)
      .flatMap(([zoneName, cards]) => cards.map(cardName => ({ playerIndex, zoneName, cardName, skipTriggers: true }))));
    assert.equal(game.addCardsToZones(placements).length, placements.length);
    game.finishPuzzleSetup();
    const state = game.uiState();
    assert.equal(state.players.length, 2);
    for (const [i, player] of state.players.entries()) {
      assert.equal(player.battlefield.reduce((total, card) => total + (card.count || 1), 0), payload.players[i].zones.battlefield.length);
      assert.equal(player.hand_size, 7);
    }
  } finally { game.free(); }
});

for (const handCount of [3, 1]) test(`opponent answers a real Mind Rot with ${handCount} cards in hand`, { timeout: 240000 }, async () => {
  const game = new WasmGame();
  try {
    assert.equal(typeof game.getDefaultSelectionCommand, 'function', 'the rebuilt engine exposes native legal choices');
    await register(game, ['Mind Rot', 'Swamp', 'Forest', 'Plains']);
    game.resetEmpty(['Alice', 'Bob'], 20);
    const spell = game.addCardToZone(0, 'Mind Rot', 'hand', true);
    for (let i = 0; i < 3; i++) game.addCardToZone(0, 'Swamp', 'battlefield', true);
    for (const name of ['Swamp', 'Forest', 'Plains'].slice(0, handCount)) game.addCardToZone(1, name, 'hand', true);
    for (let i = 0; i < 20; i++) {
      game.addCardToZone(0, 'Plains', 'library', true);
      game.addCardToZone(1, 'Plains', 'library', true);
    }
    game.finishPuzzleSetup();
    let state = game.uiState();
    let cast = false;
    let discarded = false;
    for (let step = 0; step < 80; step++) {
      // A single forced discard can be completed directly by the engine.
      if (cast && state.players[1].graveyard_size === Math.min(2, handCount)) {
        discarded = true;
        break;
      }
      const decision = state.decision;
      if (!decision) { game.advancePhase(); state = game.uiState(); continue; }
      if (decision.kind === 'select_objects' && decision.player === 1) {
        const command = await buildOpponentDecisionCommand(state, game);
        assert.equal(command.object_ids.length, Math.min(2, handCount));
        state = game.dispatch(command);
        discarded = true;
        break;
      }
      let command;
      if (decision.kind === 'priority') {
        const action = !cast && decision.actions.find(action => Number(action.action_ref?.spell_id) === Number(spell))
          || decision.actions.find(action => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(action.action_ref?.kind));
        assert.ok(action, JSON.stringify(decision));
        if (action.action_ref?.kind === 'cast_spell') cast = true;
        command = decision.player === state.perspective
          ? { type: 'priority_action', action_ref: action.action_ref }
          : await buildOpponentDecisionCommand(state, game);
      } else if (decision.kind === 'targets') command = { type: 'select_targets', targets: [{ kind: 'player', player: 1 }] };
      else if (decision.kind === 'mana_payment') {
        command = await buildOpponentDecisionCommand({ ...state, perspective: 1 }, game);
        assert.equal(command.response.action, 'confirm', 'the opponent confirms its payable plan');
      }
      else if (decision.kind === 'select_options') {
        const option = decision.options.find(option => /^Normal:/.test(option.description)) || decision.options.find(option => option.legal);
        command = { type: 'select_options', option_indices: [option.index] };
      } else assert.fail(JSON.stringify(decision));
      state = game.dispatch(command);
    }
    assert.ok(discarded, 'Mind Rot must reach and complete the opponent discard');
    assert.equal(state.players[1].hand_size, Math.max(0, handCount - 2));
    assert.equal(state.players[1].graveyard_size, Math.min(2, handCount));
  } finally { game.free(); }
});
