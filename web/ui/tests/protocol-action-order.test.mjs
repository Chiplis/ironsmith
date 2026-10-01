import test from 'node:test';
import assert from 'node:assert/strict';
import { createProtocolActionOrder } from '../src/lib/protocol-action-order.js';

function harness() {
  let seq = 261, match = 'game';
  const timers = new Set();
  const order = createProtocolActionOrder({ head: () => seq, matchId: () => match,
    setTimer: fn => { timers.add(fn); return fn; }, clearTimer: fn => timers.delete(fn) });
  return { order, timers, advance: value => { seq = value; order.notify(); },
    rematch: () => { match = 'rematch'; order.reset(); },
    request: seq => ({ seq, matchId: match }) };
}

test('263 waits for delayed 262 but requests needed by 262 still pass', async () => {
  const h = harness(); let passed = false;
  const waiting = h.order.wait(h.request(263)).then(() => { passed = true; });
  await h.order.wait(h.request(262), 'Random commitment request');
  assert.equal(passed, false);
  h.advance(262); await waiting;
  assert.equal(passed, true); assert.equal(h.timers.size, 0);
});

test('missing preceding actions time out with recovery diagnostics', async () => {
  const h = harness();
  const waiting = h.order.wait(h.request(263));
  [...h.timers][0]();
  await assert.rejects(waiting, /waiting for action 262.*accepted 261.*requested 263/);
  assert.equal(h.order.pending(), 0);
});

test('stale, malformed, and foreign requests are rejected without waiting', async () => {
  const h = harness();
  await assert.rejects(h.order.wait(h.request(261)), /expected 262, received 261/);
  await assert.rejects(h.order.wait(h.request(1.5)), /invalid action sequence/);
  await assert.rejects(h.order.wait({ seq: 263, matchId: 'other' }), /different match/);
  assert.equal(h.order.pending(), 0);
});

test('recovery cancels all waits; old-match callbacks cannot authorize a rematch', async () => {
  const h = harness();
  const waiting = h.order.wait(h.request(263));
  h.rematch();
  await assert.rejects(waiting, /cancelled/);
  assert.equal(h.timers.size, 0);
});

test('if a newer action overtakes a request during catch-up it remains stale', async () => {
  const h = harness();
  const waiting = h.order.wait(h.request(263));
  h.advance(263);
  await assert.rejects(waiting, /expected 264, received 263/);
});

test('discarding a dependent provisional action cancels its wait and timer immediately', async () => {
  const h = harness(), abort = new AbortController();
  const waiting = h.order.wait(h.request(263), 'Local action dependency', { signal: abort.signal });
  assert.equal(h.order.pending(), 1);
  abort.abort();
  await assert.rejects(waiting, /cancelled/);
  assert.equal(h.order.pending(), 0);
  assert.equal(h.timers.size, 0);
});
