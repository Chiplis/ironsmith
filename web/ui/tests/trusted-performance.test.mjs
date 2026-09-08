import test from 'node:test';
import assert from 'node:assert/strict';
import { createSnapshotEncoder, createSnapshotDecoder } from '../src/lib/snapshot-channel.js';
import { immutableAction, actionCursor, restoreActionCursor, withActionPrefixes, actionPrefixHash, EMPTY_ACTION_PREFIX } from '../src/lib/accepted-actions.js';
import { createAsyncLimiter } from '../src/lib/bounded-async.js';
import { compileWasmWithProgress } from '../src/lib/wasm-loading.js';
import { createValueStore, differsBeyondClock } from '../src/lib/value-store.js';
import { indexFormatCatalog } from '../src/lib/relay/format-catalog-index.js';

test('snapshot patches preserve unchanged entity identities and recover after missed revisions', () => {
  const encoder = createSnapshotEncoder(), decoder = createSnapshotDecoder();
  const state = { players: [{ life: 20, hand: [{ id: 1, name: 'Island' }] }, { life: 20 }], decision: { kind: 'priority' } };
  const before = decoder.decode(structuredClone(encoder.encode(state)));
  const after = { ...state, players: [{ ...state.players[0], life: 19 }, state.players[1]] };
  const patch = encoder.encode(after);
  assert.equal(patch.changes.length, 1);
  const updated = decoder.decode(structuredClone(patch));
  assert.deepEqual(updated, after);
  assert.equal(updated.players[1], before.players[1]);
  assert.equal(updated.players[0].hand, before.players[0].hand);
  encoder.encode({ ...after, decision: null });
  assert.throws(() => decoder.decode(encoder.encode({ ...after, phase: 'combat' })), /recovery/);
  assert.deepEqual(decoder.decode(encoder.encode(after, { full: true })), after);
  assert.throws(() => decoder.decode(patch), /Stale/);
});

test('snapshot replacement, removals, overflow and prototype keys round trip', () => {
  const encoder = createSnapshotEncoder({ limit: 1 }), decoder = createSnapshotDecoder();
  for (const state of [{ a: [1, 2], b: 3 }, { a: [3], c: 5 }, JSON.parse('{"__proto__":{"safe":true}}'), { a: null }]) {
    assert.deepEqual(decoder.decode(structuredClone(encoder.encode(state))), state);
  }
  assert.equal({}.safe, undefined);
});

test('accepted entries are immutable and rollback stores a cursor instead of copying history', () => {
  const source = { seq: 1, actorIndex: 0, command: { type: 'pass' } };
  const history = withActionPrefixes([source]);
  source.command.type = 'changed';
  assert.equal(history[0].command.type, 'pass');
  assert.throws(() => { history[0].command.type = 'mutation'; }, TypeError);
  const cursor = actionCursor(history);
  assert.equal(cursor.entries, history);
  history.push(immutableAction({ seq: 2 }));
  const restored = restoreActionCursor(cursor);
  assert.equal(restored.length, 1);
  assert.equal(restored[0], history[0]);
  assert.equal(history[0].prefixHash, actionPrefixHash(EMPTY_ACTION_PREFIX, history[0]));
  assert.throws(() => withActionPrefixes([{ ...history[0], command: { type: 'tampered' } }]), /mismatch/);
});

test('source preparation limits concurrent work and drains after rejection', async () => {
  const limit = createAsyncLimiter(3);
  let running = 0, high = 0;
  const results = await Promise.allSettled(Array.from({ length: 12 }, (_, n) => limit(async () => {
    high = Math.max(high, ++running);
    await new Promise(resolve => setTimeout(resolve, 1));
    running--;
    if (n === 4) throw new Error('missing');
    return n;
  })));
  assert.equal(high, 3); assert.equal(running, 0);
  assert.equal(results.filter(r => r.status === 'rejected').length, 1);
  assert.equal(await limit(() => 42), 42);
});

test('WASM progress shares the compile stream and preserves cacheable URL', async () => {
  const bytes = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);
  const calls = [], progress = [];
  const module = await compileWasmWithProgress('/engine-hash.wasm', n => progress.push(n), {
    fetchModule: async (...args) => { calls.push(args); return new Response(bytes, { headers: { 'content-length': '8' } }); },
  });
  assert.ok(module instanceof WebAssembly.Module);
  assert.deepEqual(calls, [['/engine-hash.wasm']]);
  assert.equal(progress.at(-1), 1);
});

test('WASM streaming API fallback retries once and invalid binaries propagate', async () => {
  let calls = 0;
  const fetchModule = async () => { calls++; return new Response(new Uint8Array([1])); };
  const marker = {};
  assert.equal(await compileWasmWithProgress('/cached', undefined, { fetchModule,
    wasm: { compileStreaming: async () => { throw new TypeError('unsupported'); }, compile: async () => marker } }), marker);
  assert.equal(calls, 2);
  await assert.rejects(compileWasmWithProgress('/invalid', undefined, { fetchModule,
    wasm: { compileStreaming: async () => { throw new WebAssembly.CompileError('invalid'); } } }), WebAssembly.CompileError);
  assert.equal(calls, 3);
});

test('clock store notifies only its subscribers and detects durable/semantic changes', () => {
  const clock = createValueStore(); let notifications = 0;
  const unsubscribe = clock.subscribe(() => notifications++);
  const value = { remaining: 5 }; clock.set(value); clock.set(value);
  assert.equal(notifications, 1); unsubscribe(); clock.set(null); assert.equal(notifications, 1);
  const before = { players: [], matchClock: value, submittingAction: false };
  assert.equal(differsBeyondClock(before, { ...before, matchClock: null, actionTimer: {} }), false);
  assert.equal(differsBeyondClock(before, { ...before, submittingAction: true }), true);
});

test('alias index keeps first catalog insertion for front-face collisions', () => {
  const first = { name: 'Front // Back' }, second = { name: 'Front // Other' };
  const cards = { first, second };
  const aliases = indexFormatCatalog(cards);
  assert.equal(aliases.get('front'), first);
  assert.equal(indexFormatCatalog(cards), aliases);
});
