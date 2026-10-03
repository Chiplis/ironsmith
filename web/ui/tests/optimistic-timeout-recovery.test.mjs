import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { createOptimisticMatch } from '../src/lib/optimistic-match.js';

const optimisticSource = readFileSync(new URL('../src/hooks/peer-lobby/optimistic-state.js', import.meta.url), 'utf8')
  .replace(/import[\s\S]*?from\s+['"][^'"]+['"];\s*/g, '')
  .replace('export function useOptimisticPeerState', 'function useOptimisticPeerState');
const lobbySource = readFileSync(new URL('../src/hooks/usePeerLobby.js', import.meta.url), 'utf8');
const start = lobbySource.indexOf('  function deferProtocolResponseTimeoutClaim(');
const end = lobbySource.indexOf('\n  }\n', start);
const deferSource = lobbySource.slice(start, end + 4);

function harness() {
  const states = [], calls = [], errors = [];
  let matchId = 'match', recovery = async () => { calls.push('timeout submitted'); };
  const state = { decision: { kind: 'priority', player: 0 }, perspective: 0, stack_size: 1 };
  const visible = {
    supportsRuntimeBranches: true, uiState: async () => state,
    forkRuntimeBranch: async () => ({ uiState: async () => state,
      copyToVisible: async () => state, release: async () => {} }),
  };
  const session = { current: { matchStarted: true, localPlayerIndex: 0, lastAppliedSequence: 249 } };
  const context = {
    createOptimisticMatch, setTimeout, clearTimeout, AbortController, console,
    useRef: value => ({ current: value }),
    useState: value => { const slot = states.length; states.push(value); return [value, next => { states[slot] = next; }]; },
    useEffect: () => {}, isTrustedMultiplayerSecurityMode: () => false,
    sessionSecurityMode: () => 'verified', isMatchDisputed: value => Boolean(value.matchDisputed),
    isForfeitCommand: command => command.type === 'forfeit_player', PROTOCOL_VERSION: 15,
    calculateOptimisticAction: async () => null, isDecisionCommandCompatible: () => true,
    wireStablePayload: value => value, randomAuditHex: () => 'candidate',
    canonicalMultiplayerPayload: JSON.stringify,
    recordPeerSyncPerf: () => {}, toErrorMessage: error => error.message,
    multiplayerRef: session, currentAuditMatchId: () => matchId,
    setStatus: message => errors.push(message),
  };
  vm.createContext(context);
  vm.runInContext(optimisticSource + '\nthis.hook = useOptimisticPeerState;', context);
  const base = { game: visible, gameRef: { current: visible }, stateRef: { current: state },
    multiplayerRef: session, actionHistoryRef: { current: [] }, awaitingStateResyncRef: { current: false },
    hostConnectionRef: { current: null }, clientConnectionsRef: { current: new Map() }, peerConnectionsRef: { current: new Map() },
    setState: () => {}, setStatus: context.setStatus };
  const api = context.hook(base, { current: { currentAuditMatchId: context.currentAuditMatchId,
    waitForProtocolActionHead: async () => {} } });
  context.optimisticState = api;
  context.submitProtocolResponseTimeoutClaim = () => api.stageOptimisticLocalCommand(
    { type: 'forfeit_player', player: 1 }, 'protocol timeout', () => recovery());
  vm.runInContext(deferSource, context);
  return { api, calls, errors, session, defer: () => context.deferProtocolResponseTimeoutClaim({ requestId: 'timeout' }),
    blocked: () => states[1], setRecovery: fn => { recovery = fn; }, replaceMatch: () => { matchId = 'replacement'; } };
}

const nextTurn = () => new Promise(resolve => setTimeout(resolve, 10));
const pass = { type: 'priority_action', action_ref: { kind: 'pass_priority' } };

test('timeout recovery runs after verification finally and leaves Resolve usable', async () => {
  const h = harness(); await h.api.ensureOptimisticRuntime();
  await h.api.runVerifiedTask(async () => {
    try { h.calls.push('original catch'); h.defer(); }
    finally { h.calls.push('original finally'); }
  });
  await nextTurn();
  assert.deepEqual(h.calls, ['original catch', 'original finally', 'timeout submitted']);
  assert.equal(h.blocked(), false);
  await h.api.stageOptimisticLocalCommand(pass, 'Resolve', async () => h.calls.push('Resolve submitted'));
  assert.equal(h.calls.at(-1), 'Resolve submitted');
  assert.equal(h.blocked(), false);
  assert.deepEqual(h.errors, []);
});

test('material-blocked foreground click releases its promise-race gate before recovery submits', async () => {
  const h = harness(); await h.api.ensureOptimisticRuntime();
  h.setRecovery(async () => { assert.equal(h.blocked(), true); h.calls.push('timeout submitted'); });
  await h.api.stageOptimisticLocalCommand(pass, 'Resolve', async () => {
    assert.equal(h.blocked(), true);
    h.defer(); return false;
  });
  assert.equal(h.blocked(), false, 'the foreground finally has released the old gate');
  await nextTurn();
  assert.deepEqual(h.calls, ['timeout submitted']);
  assert.equal(h.blocked(), false);
});

test('discarding the failed provisional suffix does not cancel its deferred timeout recovery', async t => {
  const h = harness(); await h.api.ensureOptimisticRuntime();
  t.after(() => h.api.resetOptimisticState('test complete'));
  await h.api.stagePreparedLocalAction({ seq: 250, actorIndex: 0, command: pass, label: 'Resolve',
    publicCheckpointHash: 'checkpoint', openings: [], rngReveals: [], shuffleProofs: [] });
  await h.api.runVerifiedTask(async () => {
    h.defer();
    await h.api.failOptimisticAction(250, 'Action was not accepted');
  });
  await nextTurn();
  assert.deepEqual(h.calls, ['timeout submitted']);
  assert.equal(h.blocked(), false);
});

test('failed deferred recovery reports the failure and releases the interaction gate', async () => {
  const h = harness(); await h.api.ensureOptimisticRuntime();
  h.setRecovery(async () => { throw Error('certificate rejected'); });
  await h.api.runVerifiedTask(async () => h.defer());
  await nextTurn();
  assert.equal(h.blocked(), false);
  assert.deepEqual(h.errors, ['Protocol timeout recovery failed: certificate rejected']);
  await h.api.stageOptimisticLocalCommand(pass, 'Resolve', async () => h.calls.push('Resolve submitted'));
  assert.deepEqual(h.calls, ['Resolve submitted']);
});

for (const transition of ['replacement', 'disputed', 'ended', 'reset']) {
  test(`deferred timeout cannot act on a ${transition} match`, async () => {
    const h = harness(); await h.api.ensureOptimisticRuntime();
    await h.api.runVerifiedTask(async () => {
      h.defer();
      if (transition === 'replacement') h.replaceMatch();
      if (transition === 'disputed') h.session.current.matchDisputed = { reason: 'fork' };
      if (transition === 'ended') h.session.current.matchStarted = false;
      if (transition === 'reset') await h.api.resetOptimisticState('replaced');
    });
    await nextTurn();
    assert.deepEqual(h.calls, []);
    assert.equal(h.blocked(), false);
    assert.deepEqual(h.errors, []);
  });
}
