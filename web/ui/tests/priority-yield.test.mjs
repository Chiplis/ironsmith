import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { isDecisionCommandCompatible, isOwnPassCancellation, resolveSyncedCommand } from '../src/lib/sync-commands.js';

const priority = { kind: 'priority', player: 1, actions: [{ kind: 'pass_priority', label: 'Pass priority', action_ref: { kind: 'pass_priority' } }] };

test('shared passing is a replayable intention, only available at ordinary priority', () => {
  for (const mode of ['stack', 'phases']) {
    const command = { type: 'priority_yield', mode, stops: ['phase:Main2', 'step:DeclareAttackers'] };
    assert.equal(isDecisionCommandCompatible(priority, command), true);
    assert.deepEqual(resolveSyncedCommand(JSON.parse(JSON.stringify(command))), command);
    for (const kind of ['attackers', 'blockers', 'number', 'select_options', 'mana_payment', 'targets']) {
      assert.equal(isDecisionCommandCompatible({ ...priority, kind }, command), false, kind);
    }
    assert.equal(isDecisionCommandCompatible({ ...priority, actions: [{ kind: 'pass_priority', label: 'Keep hand' }] }, command), false);
  }
});

test('out of turn cancellation authorizes only the signed seat', () => {
  const command = { type: 'cancel_priority_yield', player: 0 };
  assert.equal(isDecisionCommandCompatible(priority, command), true);
  assert.equal(isOwnPassCancellation(command, 0), true);
  for (const actor of [1, null, undefined]) assert.equal(isOwnPassCancellation(command, actor), false);
  for (const player of [-1, 256, 0.5, '0', null, undefined]) {
    assert.equal(isOwnPassCancellation({ ...command, player }, 0), false);
  }
});

// Exercise the actual receiving validation, including its out-of-turn gate.
const source = readFileSync(new URL('../src/hooks/peer-lobby/crypto-resync.js', import.meta.url), 'utf8');
const start = source.indexOf('  async function validateTrustedSequencedAction(');
const end = source.indexOf('\n  }\n', start) + 4;
const validate = new Function('isOwnPassCancellation', 'isDecisionCommandCompatible', `
  const normalizePlayerIndex = value => Number.isInteger(value) && value >= 0 && value < 2 ? value : null;
  const MATCH_CLOCK_CLAIM_SKEW_MS = 0, MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS = 0;
  const stateRef = { current: null };
  const multiplayerRef = { current: { players: [] } };
  const matchClockObservationExemptSequenceRef = { current: 0 };
  const isUnauthorizedAddCardCommand = () => false;
  const isActionTimeoutForfeitCommand = () => false;
  const isDisconnectTimeoutForfeitCommand = () => false;
  const isProtocolResponseTimeoutForfeitCommand = () => false;
  const isSelfForfeitCommand = () => false;
  const isForfeitCommand = () => false;
  const recordDiagnosticEvent = () => {};
  const verifyMatchClockAuditForAction = async () => {};
  ${source.slice(start, end)}
  return validateTrustedSequencedAction;
`)(isOwnPassCancellation, isDecisionCommandCompatible);

test('receiver accepts own cancellation out of turn and rejects cancelling another seat', async () => {
  await validate({ command: { type: 'cancel_priority_yield', player: 0 }, actorIndex: 0, uiState: { decision: priority } });
  await assert.rejects(validate({ command: { type: 'cancel_priority_yield', player: 0 }, actorIndex: 1, uiState: { decision: priority } }), /only cancel their own/);
  await assert.rejects(validate({ command: { type: 'priority_yield', mode: 'phases', stops: [] }, actorIndex: 0, uiState: { decision: priority } }), /not the current decision player/);
  await validate({ command: { type: 'priority_yield', mode: 'phases', stops: [] }, actorIndex: 1, uiState: { decision: priority } });
});
