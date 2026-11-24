import test from 'node:test';
import assert from 'node:assert/strict';
import { createHeadlessHook, useCallback, useEffect, useRef, useState } from '../src/lib/verified-session/hook-runtime.js';

test('headless hooks keep state, refs and stable callbacks across renders', async () => {
  let renders = 0;
  const hook = createHeadlessHook(({ step }) => {
    renders += 1;
    const [count, setCount] = useState(() => 10);
    const ref = useRef({ created: renders });
    const bump = useCallback(() => setCount(value => value + step), [step]);
    return { count, bump, ref: ref.current, renders };
  }, { step: 1 });
  const first = hook.current;
  assert.equal(first.count, 10);
  first.bump(); first.bump();
  await hook.settled();
  assert.equal(hook.current.count, 12, 'updaters compose and coalesce into one render');
  assert.equal(hook.current.renders, 2);
  assert.equal(hook.current.ref, first.ref, 'refs persist');
  assert.equal(hook.current.bump, first.bump, 'callback identity stable while deps are unchanged');
  hook.update({ step: 5 });
  assert.notEqual(hook.current.bump, first.bump, 'callback changes with its deps');
  hook.current.bump();
  await hook.settled();
  assert.equal(hook.current.count, 17);
  hook.dispose();
});

test('effects run after the render that changed their deps, cleanup first, and on dispose', async () => {
  const log = [];
  const hook = createHeadlessHook(({ id }) => {
    const [tick, setTick] = useState(0);
    useEffect(() => { log.push(`mount:${id}`); return () => log.push(`cleanup:${id}`); }, [id]);
    useEffect(() => { log.push(`every:${tick}`); });
    useEffect(() => { if (tick === 0) setTick(1); }, [tick]);
    return { tick };
  }, { id: 'a' });
  await hook.settled();
  assert.deepEqual(log, ['mount:a', 'every:0', 'every:1']);
  hook.update({ id: 'b' });
  assert.deepEqual(log.slice(3), ['cleanup:a', 'mount:b', 'every:1']);
  hook.dispose();
  assert.equal(log.at(-1), 'cleanup:b');
  await hook.settled();
  assert.equal(log.filter(entry => entry.startsWith('cleanup')).length, 2, 'no work after dispose');
});

test('setting the same state value does not re-render and listeners see each commit', async () => {
  let setter;
  let renders = 0;
  const hook = createHeadlessHook(() => { renders += 1; const [v, set] = useState('x'); setter = set; return v; }, {});
  const seen = [];
  hook.subscribe(value => seen.push(value));
  setter('x');
  await hook.settled();
  assert.equal(renders, 1);
  setter('y');
  await hook.settled();
  assert.deepEqual(seen, ['y']);
  hook.dispose();
});
