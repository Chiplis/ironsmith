import test from 'node:test';
import assert from 'node:assert/strict';
import { createPriorityPolicy } from '../src/lib/priority-policy.js';
import { readGoldfishSeats, storeGoldfishSeats } from '../src/lib/play-preferences.js';
const memory = () => {
 const values = new Map();
 return { getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) };
};
const state = { perspective: 0, turn_number: 1 };
test('Hold is the fresh default; chosen modes survive policy recreation and game resets', () => {
 const storage = memory();
 let policy = createPriorityPolicy({ storage });
 assert.equal(policy.status(state).stackMode, 'hold');
 for (const mode of ['countdown', 'pass', 'hold']) {
  policy.setStackMode(mode, state);
  policy = createPriorityPolicy({ storage });
  assert.equal(policy.status(state).stackMode, mode);
  policy.reset();
  assert.equal(policy.status(state).stackMode, mode);
 }
 policy.setStackMode('pass', state);
 assert.equal(policy.status({ ...state, perspective: 1 }).stackMode, 'hold');
});
test('goldfish seat toggles, including all off, survive a new session', () => {
 const storage = memory();
 assert.deepEqual([...readGoldfishSeats(storage)], [1]);
 for (const seats of [[], [1, 2], [2]]) {
  storeGoldfishSeats(new Set(seats), storage);
  assert.deepEqual([...readGoldfishSeats(storage)], seats);
 }
});
test('invalid or unavailable storage preserves defaults and usable controls', () => {
 for (const storage of [null, {getItem:()=>'{broken',setItem:()=>{throw Error('blocked');}},
  {getItem:()=>'{"0":"unknown"}',setItem:()=>{throw Error('blocked');}}]) {
  const policy = createPriorityPolicy({storage});
  assert.equal(policy.status(state).stackMode, 'hold');
  policy.setStackMode('countdown', state);
  assert.equal(policy.status(state).stackMode, 'countdown');
  assert.deepEqual([...readGoldfishSeats(storage)], [1]);
  assert.doesNotThrow(()=>storeGoldfishSeats(new Set(), storage));
 }
});
