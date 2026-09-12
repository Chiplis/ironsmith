import test from 'node:test';
import assert from 'node:assert/strict';
import { createFrameStore } from '../src/lib/frame-store.js';

test('pointer bursts notify once per frame and release reads the latest sample', () => {
  const frames = new Map();
  let id = 0;
  const store = createFrameStore((fn) => { frames.set(++id, fn); return id; }, (key) => frames.delete(key));
  let notifications = 0;
  const unsubscribe = store.subscribe(() => notifications++);
  for (let x = 0; x < 100; x++) store.set({ x, y: 20 });
  assert.equal(frames.size, 1);
  assert.deepEqual(store.getSnapshot(), { x: 99, y: 20 });
  assert.equal(notifications, 0);
  const frame = frames.values().next().value;
  frames.clear();
  frame();
  assert.equal(notifications, 1);
  store.set(null);
  assert.equal(store.getSnapshot(), null);
  unsubscribe();
  store.dispose();
  assert.equal(frames.size, 0);
  assert.equal(notifications, 1);
});
