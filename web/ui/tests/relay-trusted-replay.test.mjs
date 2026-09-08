import test from 'node:test';
import assert from 'node:assert/strict';
import { replayTrustedMatch } from '../src/lib/relay/replay-trusted-match.js';

const action = { type: 'priority_action', action_ref: { kind: 'play_land', land_id: 42 } };
function fixture(complete, dispatch = async () => {}) {
  return {
    startMatch: async () => {}, setPerspective: async () => {},
    uiState: async () => ({ decision: { kind: 'priority', analysis_complete: complete, actions: [] } }),
    dispatch,
  };
}
test('replay lets the engine resolve stable actions missing from incomplete priority analysis', async () => {
  const commands = [];
  await replayTrustedMatch(fixture(false, async command => commands.push(command)), {}, [{ seq: 1, command: action }], 0);
  assert.deepEqual(commands, [action]);
});
test('replay preserves engine rejection and identifies the failed sequence', async () => {
  await assert.rejects(replayTrustedMatch(fixture(false, async () => { throw new Error('invalid priority action ref'); }), {}, [{ seq: 1, command: action }], 0), /Could not restore action 1: invalid priority action ref/);
});
test('completed action lists and index-only commands remain strictly checked', async () => {
  for (const [complete, command] of [[true, action], [false, { type: 'priority_action', action_index: 9 }]]) {
    await assert.rejects(replayTrustedMatch(fixture(complete, () => assert.fail('must not dispatch')), {}, [{ seq: 1, command }], 0), /Could not restore action 1/);
  }
});
