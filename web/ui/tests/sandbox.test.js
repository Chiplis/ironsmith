import test from 'node:test';
import assert from 'node:assert/strict';
import { SANDBOX_RULES, sandboxerDraft, buildSandboxPlan, sandboxCatalogNames, startSandbox } from '../src/lib/sandbox.js';
const draft = { layout: 'single', hasFuse: false, faces: [{ name: 'Test Adept', manaCost: '{2}{U}{R}', cardTypes: ['Creature'], oracleText: '', power: '2', toughness: '2' }] };
const input = { selections: [{ id: 'test', draft, copies: 4 }], testedId: 'test' };
test('every rules toggle contributes exactly its own Sandboxer line', () => {
  assert.equal(sandboxerDraft().faces[0].oracleText, '');
  for (const rule of SANDBOX_RULES) {
    assert.equal(sandboxerDraft({ [rule.id]: true }).faces[0].oracleText, rule.text);
    const all = Object.fromEntries(SANDBOX_RULES.map(item => [item.id, true]));
    all[rule.id] = false;
    assert.equal(sandboxerDraft(all).faces[0].oracleText.split('\n').length, SANDBOX_RULES.length - 1);
    assert.ok(!sandboxerDraft(all).faces[0].oracleText.includes(rule.text));
  }
});
test('default decks have 60 cards, colored basics, seven-card hands and exactly four test copies', () => {
  const plan = buildSandboxPlan(input, () => 0.5);
  assert.equal(plan.players.length, 2);
  for (const player of plan.players) { assert.equal(player.hand.length, 7); assert.equal(player.library.length, 53); }
  assert.equal(plan.players[0].hand[0].key, 'test');
  const all = [...plan.players[0].hand, ...plan.players[0].library];
  assert.equal(all.filter(card => card.key === 'test').length, 4);
  assert.equal(all.filter(card => ['Island', 'Mountain'].includes(card.name)).length, 24);
  assert.deepEqual(new Set(sandboxCatalogNames(plan)), new Set(['Island', 'Mountain', 'Coral Merfolk', 'Goblin Piker']));
});
test('custom decks and arbitrary copy counts retain a guaranteed test card', () => {
  const plan = buildSandboxPlan({ ...input, selections: [{ ...input.selections[0], copies: 9 }], deck: Array(10).fill('Forest'), opponentDeck: Array(8).fill('Plains'), startingLife: 40, options: { graveyard: true }, bothPlayers: false });
  assert.equal(plan.players[0].hand.length + plan.players[0].library.length, 19);
  assert.equal(plan.players[1].library.length, 1);
  assert.equal(plan.players[0].hand[0].draft.faces[0].name, 'Test Adept');
  assert.equal(plan.startingLife, 40);
  assert.equal(plan.sandboxer.faces[0].oracleText, SANDBOX_RULES.find(rule => rule.id === 'graveyard').text);
  assert.equal(plan.bothPlayers, false);
  for (const copies of [0, -1, 1.5, '']) assert.throws(() => buildSandboxPlan({ ...input, selections: [{ ...input.selections[0], copies }] }), /Copies/);
  assert.throws(() => buildSandboxPlan({ ...input, opponentDeck: ['Island'] }), /7 to/);
});
function fakeGame(failAt) {
  let board = ['old table'], counter = 0, old;
  return {
    get board() { return board; },
    createRuntimeSavepoint() { old = [...board]; return 1; },
    restoreRuntimeSavepoint() { board = old; }, releaseRuntimeSavepoint() {},
    previewCustomCard() { return { canCreate: true }; },
    resetEmpty() { board = []; }, setPerspective() {}, finishPuzzleSetup() {},
    createCustomCard({ draft, playerIndex, zoneName }) { return this.addCardToZone(playerIndex, draft.faces[0].name, zoneName); },
    addCardToZone(player, name, zone) { if (++counter === failAt) throw new Error('Insertion failed'); board.push({ player, name, zone }); return counter; },
  };
}
test('worker setup uses real card placement and creates only requested Sandboxers', async () => {
  for (const bothPlayers of [true, false]) {
    const game = fakeGame();
    await startSandbox(game, buildSandboxPlan({ ...input, options: { freeCasting: true }, bothPlayers }));
    assert.equal(game.board.filter(card => card.name === 'IronSmith Sandboxer').length, bothPlayers ? 2 : 1);
    assert.equal(game.board.filter(card => card.zone === 'hand').length, 14);
  }
});
test('failed setup restores the original table', async () => {
  const game = fakeGame(10);
  await assert.rejects(startSandbox(game, buildSandboxPlan(input)), /Insertion failed/);
  assert.deepEqual(game.board, ['old table']);
});
