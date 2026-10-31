import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { buildOpponentDecisionCommand } from '../src/lib/opponent-decision.js';
import { isAutomatedSeat } from '../src/lib/hotseat-follow.js';
const source = readFileSync(new URL('../src/context/GameContext.jsx', import.meta.url), 'utf8');
const start = source.indexOf('async (currentGame, currentState) => {', source.indexOf('const settlePriorityAutomation ='));
const end = source.indexOf('}, [priorityPolicy, autoPassEnabled]);', start) + 1;
function settle() {
  return new Function('multiplayerActiveRef', 'automatedSeatsRef', 'autoPassEnabled',
    'isAutomatedSeat', 'buildOpponentDecisionCommand', 'summarizeDecision', 'dispatchForAutomation',
    `return (${source.slice(start, end)});`)(
    { current: false }, { current: new Set([1]) }, false,
    isAutomatedSeat, buildOpponentDecisionCommand, x => x,
    (game, command) => game.dispatch(command));
}
for (const perspective of [0, 1]) test(`goldfish discards with auto-pass off (viewing ${perspective})`, async () => {
  const state = { perspective, active_player: 1, priority_player: 0, step: 'Cleanup',
    players: [{ id: 0 }, { id: 1 }], decision: { kind: 'select_objects', player: 1,
      min: 2, max: 2, candidates: [{ id: 8, legal: false }, { id: 9 }, { id: 10 }, { id: 11 }] } };
  const commands = [];
  const result = await settle()({ dispatch: async command => {
    commands.push(command);
    return { ...state, decision: null, game_over: true };
  } }, state);
  assert.deepEqual(commands, [{ type: 'select_objects', object_ids: [9, 10] }]);
  assert.equal(result.state.decision, null);
});
test('goldfish uses native legal selections and leaves human choices alone', async () => {
  const state = { perspective: 1, decision: { kind: 'select_objects', player: 1 } };
  const command = { type: 'select_objects', object_ids: [55] };
  const game = { getDefaultSelectionCommand: async () => command };
  assert.equal(await buildOpponentDecisionCommand(state, game), null);
  assert.equal(await buildOpponentDecisionCommand(state, game, { automated: true }), command);
});
const shared = readFileSync(new URL('../src/hooks/peer-lobby/shared.js', import.meta.url), 'utf8');
function clockFunction(name, dependencies = {}) {
  const begin = shared.indexOf(`export function ${name}(`);
  const finish = shared.indexOf('\n}', begin) + 2;
  return new Function(...Object.keys(dependencies), `return (${shared.slice(begin + 7, finish)});`)(...Object.values(dependencies));
}
const clockPlayer = clockFunction('matchClockActivePlayerFromState');
const debit = clockFunction('debitMatchClockRemaining', {
  normalizeMatchClockRemaining: clockFunction('normalizeMatchClockRemaining'),
});
test('cleanup charges Bob for discarding despite stale priority pointing at Alice', () => {
  const state = { active_player: 1, priority_player: 0, step: 'Cleanup', decision: { kind: 'select_objects', player: 1 } };
  assert.equal(clockPlayer(state), 1);
  assert.deepEqual(debit([10000, 10000], clockPlayer(state), 1500), [10000, 8500]);
  assert.equal(clockPlayer({ ...state, decision: null }), null);
  assert.equal(clockPlayer({ ...state, game_over: true }), null);
  assert.equal(clockPlayer({ ...state, decision: { kind: 'priority', player: 0 } }), 0);
});

