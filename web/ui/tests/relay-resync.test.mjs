import test from 'node:test';
import assert from 'node:assert/strict';
import { localActionsForRelayRepair, needsFullStateResync } from '../src/lib/relay/resync.js';

test('an explicit repair request sends state even when sequences match', () => {
  assert.equal(needsFullStateResync({
    forceCheckpoint: true,
    connected: true,
    requesterSequence: 50,
    hostSequence: 50,
  }), true);
});

test('a healthy peer at the host sequence only needs an acknowledgement', () => {
  assert.equal(needsFullStateResync({
    connected: true,
    requesterSequence: 50,
    hostSequence: 50,
  }), false);
});

test('disconnected, behind, or invalid peers receive the full state', () => {
  assert.equal(needsFullStateResync({ connected: false, requesterSequence: 50, hostSequence: 50 }), true);
  assert.equal(needsFullStateResync({ connected: true, requesterSequence: 49, hostSequence: 50 }), true);
  assert.equal(needsFullStateResync({ connected: true, requesterSequence: NaN, hostSequence: 50 }), true);
});

test('an ahead peer must reconcile with the host rather than receive an ACK', () => {
  assert.equal(needsFullStateResync({ requesterSequence: 51, hostSequence: 50 }), true);
  assert.equal(needsFullStateResync({ requesterSequence: -1, hostSequence: 0 }), true);
});

test('relay recovery keeps every local action in a long effect chain', () => {
  const actions = Array.from({ length: 80 }, (_, index) => ({
    seq: index + 1,
    actorIndex: index % 3 === 0 ? 1 : 0,
  }));
  const local = localActionsForRelayRepair(actions, 0);
  assert.equal(local.length, 53);
  assert.equal(local[0].seq, 2);
  assert.equal(local.at(-1).seq, 80);
});
