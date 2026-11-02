import test from 'node:test';
import assert from 'node:assert/strict';
import { prepareShaderProgram } from '../src/lib/shader-program.js';

function harness({ extension = true, linked = true } = {}) {
  let complete = false, lost = false, next = 0;
  const frames = new Map(), calls = [];
  globalThis.requestAnimationFrame = fn => { frames.set(++next, fn); return next; };
  globalThis.cancelAnimationFrame = id => frames.delete(id);
  const gl = {
    VERTEX_SHADER: 1, FRAGMENT_SHADER: 2, LINK_STATUS: 3,
    createProgram: () => ({}), createShader: () => ({}),
    getExtension: () => extension ? { COMPLETION_STATUS_KHR: 4 } : null,
    shaderSource() {}, compileShader() { calls.push('compile'); }, attachShader() {},
    linkProgram() { calls.push('link'); }, isContextLost: () => lost,
    getProgramParameter(_, key) { calls.push(key); return key === 4 ? complete : linked; },
    getProgramInfoLog: () => 'link error', getShaderInfoLog: () => 'shader error',
    deleteShader() { calls.push('deleteShader'); }, deleteProgram() { calls.push('deleteProgram'); },
  };
  return { gl, calls, frames, complete() { complete = true; }, lose() { lost = true; },
    tick() { const pending = [...frames.values()]; frames.clear(); pending.forEach(fn => fn()); } };
}

test('defers link status until completion and submits both shaders first', async () => {
  const h = harness(), job = prepareShaderProgram(h.gl, 'v', 'f');
  assert.deepEqual(h.calls, ['compile', 'compile', 'link']);
  h.tick(); h.tick(); assert.ok(!h.calls.includes(3));
  h.complete(); h.tick(); assert.equal(await job.ready, true); job.dispose();
});
test('missing extension checks link on next frame', async () => {
  const h = harness({ extension: false }), job = prepareShaderProgram(h.gl, 'v', 'f');
  assert.ok(!h.calls.includes(3)); h.tick();
  assert.equal(await job.ready, true); job.dispose();
});
test('link failures report diagnostics and remain disposable', async () => {
  const h = harness({ linked: false }), job = prepareShaderProgram(h.gl, 'v', 'f');
  h.complete(); h.tick();
  await assert.rejects(job.ready, /link error.*\nshader error/); job.dispose();
  assert.equal(h.calls.filter(call => call === 'deleteShader').length, 2);
});
test('dispose cancels polling and settles readiness', async () => {
  const h = harness(), job = prepareShaderProgram(h.gl, 'v', 'f');
  job.dispose(); job.dispose(); assert.equal(await job.ready, false);
  assert.equal(h.frames.size, 0);
  assert.equal(h.calls.filter(call => call === 'deleteProgram').length, 1);
});
test('context loss ends polling without querying link status', async () => {
  const h = harness(), job = prepareShaderProgram(h.gl, 'v', 'f');
  h.lose(); h.tick(); assert.equal(await job.ready, false);
  assert.ok(!h.calls.includes(3)); job.dispose();
});
