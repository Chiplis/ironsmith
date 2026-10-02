import test from 'node:test';
import assert from 'node:assert/strict';
import { createSequencedActionRecovery } from '../src/lib/sequenced-action-recovery.js';
function harness() {
  let head = 218, match = 'match', next = 0;
  const timers = new Map(), sent = [], applied = [], failures = [], waits = [];
  const recovery = createSequencedActionRecovery({
    head: () => head, stateHash: () => `hash-${head}`, matchId: () => match,
    requestId: () => `request-${++next}`,
    setTimer: fn => { const id = ++next; timers.set(id, fn); return id; },
    clearTimer: id => timers.delete(id),
    send: (actor, message) => sent.push({ actor, ...message }),
    apply: async action => { assert.equal(action.seq, head + 1); applied.push(action); head = action.seq; },
    onWait: wait => waits.push(wait), onFailure: failure => failures.push(failure),
  });
  const actions = (...sequences) => sequences.map(seq => ({ seq, actorIndex: 1,
    command: { type: 'select_objects', object_ids: [] }, audit: { prevStateHash: `hash-${seq - 1}` } }));
  const response = values => ({ requestId: sent.at(-1).requestId, matchId: match, actions: values });
  return { recovery, timers, sent, applied, failures, waits, actions, response,
    setMatch: value => { match = value; }, advance: () => { const [id, fn] = timers.entries().next().value; timers.delete(id); fn(); } };
}
test('missing signed action is requested and verified before subsequent actions', async () => {
  const h = harness(); h.recovery.request({ seq: 220, actorIndex: 1 });
  h.recovery.request({ seq: 221, actorIndex: 1 });
  assert.equal(h.sent.length, 1);
  assert.equal(h.sent[0].fromSequence, 219);
  await h.recovery.receive(h.response(h.actions(219, 220)));
  assert.deepEqual(h.applied.map(a => a.seq), [219, 220]);
  assert.equal(h.sent.at(-1).fromSequence, 221);
  await h.recovery.receive(h.response(h.actions(221)));
  assert.equal(h.recovery.pending(), false); assert.equal(h.timers.size, 0);
});
test('foreign, stale, skipped, and incorrectly anchored responses do not touch the engine', async () => {
  const h = harness(); h.recovery.request({ seq: 221, actorIndex: 1 });
  for (const response of [
    { ...h.response(h.actions(219)), matchId: 'other' },
    { ...h.response(h.actions(219)), requestId: 'stale' },
    h.response(h.actions(220)), h.response(h.actions(219, 221)),
    h.response([{ ...h.actions(219)[0], audit: { prevStateHash: 'wrong' } }]),
  ]) assert.equal(await h.recovery.receive(response), false);
  assert.equal(h.applied.length, 0); assert.equal(h.recovery.pending(), true);
});
test('silence ends in an explicit bounded failure without clearing the match or reopening choices', () => {
  const h = harness(); h.recovery.request({ seq: 221, actorIndex: 1 });
  h.advance(); h.advance(); h.advance();
  assert.equal(h.sent.length, 3); assert.equal(h.failures.length, 1);
  assert.equal(h.recovery.pending(), true, 'match stays paused');
  assert.equal(h.applied.length, 0); assert.equal(h.timers.size, 0);
});
test('a deterministic validation failure pauses instead of accepting the peer checkpoint', async () => {
  const failures = [];
  const recovery = createSequencedActionRecovery({ head: () => 218, stateHash: () => 'hash-218',
    matchId: () => 'match', requestId: () => 'id', send() {},
    apply: async () => { throw Error('wrong decision'); }, onFailure: f => failures.push(f) });
  recovery.request({ seq: 221, actorIndex: 1 });
  await recovery.receive({ requestId: 'id', matchId: 'match', checkpoint: { trusted: true },
    actions: [{ seq: 219, audit: { prevStateHash: 'hash-218' } }] });
  assert.equal(failures[0].error, 'wrong decision'); assert.equal(recovery.pending(), true);
  recovery.reset();
});
test('late recovery replies cannot mutate a replacement match', async () => {
  const h = harness(); h.recovery.request({ seq: 221, actorIndex: 1 });
  const response = h.response(h.actions(219)); h.setMatch('new');
  assert.equal(await h.recovery.receive(response), false); assert.equal(h.applied.length, 0);
  h.recovery.notify(); assert.equal(h.recovery.pending(), false);
});

test('a closed transport still reaches the bounded recovery failure', () => {
  const timers = [], failures = [];
  const recovery = createSequencedActionRecovery({ head: () => 218, stateHash: () => 'hash-218',
    matchId: () => 'match', requestId: () => 'id', send: () => { throw Error('closed'); },
    apply: async () => assert.fail('no action was received'),
    setTimer: fn => { timers.push(fn); return timers.length; }, clearTimer() {},
    onFailure: failure => failures.push(failure) });
  assert.doesNotThrow(() => recovery.request({ seq: 220, actorIndex: 1 }));
  for (let attempt = 0; attempt < 3; attempt++) timers[attempt]();
  assert.equal(failures.length, 1);
  assert.equal(failures[0].sequence, 219);
  assert.equal(recovery.pending(), true);
});
