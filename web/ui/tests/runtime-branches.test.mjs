import test from 'node:test';
import assert from 'node:assert/strict';
import { inRuntimeBranch, attachRuntimeBranches } from '../src/lib/runtime-branches.js';

test('background calls preserve both runtime continuations across interleaved visible calls', async () => {
  let active = { turn: 9, delayed: ['trigger'], shield: 2 };
  const saved = new Map([[1, structuredClone(active)]]);
  const game = { exchangeRuntimeSavepoint(handle) {
    const other = saved.get(handle);
    assert.ok(other);
    saved.set(handle, active); active = other;
  } };
  active.shield = 0; // A provisional action consumed the visible shield.
  await inRuntimeBranch(game, 1, () => { assert.equal(active.shield, 2); active.turn++; });
  assert.equal(active.turn, 9);
  assert.equal(active.shield, 0);
  active.delayed.push('dependent');
  await inRuntimeBranch(game, 1, () => {
    assert.equal(active.turn, 10);
    assert.deepEqual(active.delayed, ['trigger']);
  });
  assert.deepEqual(active.delayed, ['trigger', 'dependent']);
});

test('a rejected background call restores the visible branch before the queue continues', async () => {
  let active = 'visible', other = 'verified';
  const game = { exchangeRuntimeSavepoint() { [active, other] = [other, active]; } };
  await assert.rejects(inRuntimeBranch(game, 1, async () => {
    active = 'failed sandbox'; throw Error('invalid proof');
  }), /invalid proof/);
  assert.equal(active, 'visible');
  assert.equal(other, 'failed sandbox');
});

test('branch proxy routes background calls, copies losslessly, and releases its handle exactly once', async () => {
  const calls = [];
  const proxy = {};
  attachRuntimeBranches(proxy, {
    ready: () => true,
    call: async (method, args, handle) => { calls.push({ method, args, handle }); return 7; },
    createProxy: call => ({ dispatch: command => call('dispatch', [command]) }),
  });
  const branch = await proxy.forkRuntimeBranch();
  await branch.dispatch({ type: 'priority_action' });
  await branch.copyToVisible();
  await branch.release(); await branch.release();
  assert.deepEqual(calls.map(c => [c.method, c.handle]), [
    ['createRuntimeSavepoint', undefined], ['dispatch', 7],
    ['copyRuntimeSavepoint', undefined], ['releaseRuntimeSavepoint', undefined],
  ]);
  await assert.rejects(branch.dispatch({}), /released/);
  assert.equal(branch.isCurrentSnapshot({}), false);
});

test('instance replacement expires branch calls and prevents old handles freeing new allocations', async () => {
  const calls=[], proxy={runtimeGeneration:0};
  attachRuntimeBranches(proxy,{ready:()=>true,call:async(method)=>{calls.push(method);return 1;},createProxy:call=>({dispatch:()=>call('dispatch',[])})});
  const branch=await proxy.forkRuntimeBranch();
  proxy.runtimeGeneration++;
  await assert.rejects(branch.dispatch(),/expired/);
  await assert.rejects(branch.copyToVisible(),/expired/);
  await branch.release();
  assert.deepEqual(calls,['createRuntimeSavepoint']);
});
