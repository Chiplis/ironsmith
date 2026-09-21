import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readdirSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { withActionPrefixes, actionPrefixHash, wireStablePayload, EMPTY_ACTION_PREFIX } from '../src/lib/accepted-actions.js';

// PeerJS data channels use BinaryPack, which encodes an `undefined` object
// property as `null`; JSON canonicalization drops it instead. Use the real
// codec when the pnpm store has it, otherwise a faithful stand-in.
async function loadWire() {
  const store = path.resolve('node_modules/.pnpm');
  const dir = existsSync(store) ? readdirSync(store).find(name => name.startsWith('peerjs-js-binarypack@')) : null;
  const file = dir ? path.join(store, dir, 'node_modules/peerjs-js-binarypack/dist/binarypack.mjs') : null;
  if (file && existsSync(file)) {
    const { pack, unpack } = await import(pathToFileURL(file).href);
    return value => unpack(pack(value));
  }
  const nullify = value => Array.isArray(value) ? value.map(item => item === undefined ? null : nullify(item))
    : value && typeof value === 'object' ? Object.fromEntries(Object.entries(value).map(([key, item]) => [key, item === undefined ? null : nullify(item)]))
    : value;
  return nullify;
}

// The shape GameContext produces for a mana payment without a plan id.
const manaPayment = {
  type: 'mana_payment',
  response: { plan_id: undefined, request_hash: undefined, required_source_ids: ['7'], excluded_source_ids: [], preserved_source_ids: [] },
};

test('an accepted action hashed with undefined fields does not survive the peer wire', async () => {
  const wire = await loadWire();
  const hostEntry = withActionPrefixes([{ seq: 1, actorIndex: 0, command: manaPayment, clock: null }])[0];
  const received = wire({ ...hostEntry });
  assert.equal(received.command.response.plan_id, null);
  assert.throws(() => withActionPrefixes([received]), /Accepted transcript prefix mismatch/);
  assert.notEqual(actionPrefixHash(EMPTY_ACTION_PREFIX, received), hostEntry.prefixHash);
});

test('wire-stable entries hash identically on the host and on every receiving peer', async () => {
  const wire = await loadWire();
  const entries = [
    { seq: 1, actorIndex: 0, command: wireStablePayload(manaPayment), clock: null },
    { seq: 2, actorIndex: 1, command: wireStablePayload({ type: 'priority_action', action_ref: { kind: 'pass', target: undefined } }), clock: { seq: 2, clockHash: 'a'.repeat(64), remainingMsByPlayer: [1, 2] } },
  ];
  const hostHistory = withActionPrefixes(entries);
  assert.equal('plan_id' in hostHistory[0].command.response, false);
  const guestHistory = withActionPrefixes(hostHistory.map(entry => wire({ ...entry })));
  assert.deepEqual(guestHistory.map(entry => entry.prefixHash), hostHistory.map(entry => entry.prefixHash));
  // A second hop (guest re-sending its stored copy, or a resync payload) is stable too.
  const rehop = wire({ ...guestHistory[1] });
  assert.equal(actionPrefixHash(guestHistory[0].prefixHash, rehop), hostHistory[1].prefixHash);
});

test('wireStablePayload keeps null, arrays and nested values and drops only undefined properties', () => {
  assert.equal(wireStablePayload(undefined), undefined);
  assert.equal(wireStablePayload(null), null);
  assert.deepEqual(wireStablePayload({ a: null, b: undefined, c: [1, undefined, { d: undefined, e: 0 }] }), { a: null, c: [1, null, { e: 0 }] });
});

test('exile action matches the live WASM form after peer transport and resync', async () => {
  const { isDecisionCommandCompatible } = await import('../src/lib/sync-commands.js');
  const wire = await loadWire();
  const action = { index: 5, action_ref: {
    kind: 'cast_spell', spell_id: 214, from_zone: 'exile',
    casting_method: { kind: 'play_from', source: 212, zone: 'exile', use_alternative: undefined },
  } };
  const decision = { kind: 'priority', player: 0, actions: [action] };
  const entry = withActionPrefixes([{ seq: 162, actorIndex: 0, clock: null,
    command: wireStablePayload({ type: 'priority_action', action_ref: action.action_ref }),
  }])[0];
  const received = wire(entry);
  const resynced = wire(JSON.parse(JSON.stringify(received)));
  for (const copy of [entry, received, resynced]) {
    assert.equal(isDecisionCommandCompatible(decision, copy.command), true);
    assert.equal(actionPrefixHash(EMPTY_ACTION_PREFIX, copy), entry.prefixHash);
  }
});
