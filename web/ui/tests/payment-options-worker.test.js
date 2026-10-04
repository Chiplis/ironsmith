import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { createLocalAnalysisReplica } from '../src/lib/local-analysis-replay.js';

test('payment worker incrementally replays definitions and restores the canonical native state', async () => {
  const messages = [], games = [], registered = [];
  class Game {
    constructor() { games.push(this); this.points = new Map(); this.nextHandle = 0; }
    initializeRuntimeIdentityOrigin() { this.state = {}; }
    setState(value, perspective) { this.state = { value, perspective }; }
    registerDefinition(source) { registered.push(source); }
    createRuntimeSavepoint() { const h = ++this.nextHandle; this.points.set(h, structuredClone(this.state)); return h; }
    exchangeRuntimeSavepoint(h) { const before = this.state; this.state = this.points.get(h); this.points.set(h, before); }
    releaseRuntimeSavepoint(h) { return this.points.delete(h); }
    getPaymentActivationOptions(request) {
      const state = structuredClone(this.state);
      this.state.value = 'speculative payment';
      return { request, state };
    }
    free() { this.freed = true; }
  }
  const self = { postMessage(message) { messages.push(message); } };
  const code = readFileSync(new URL('../src/workers/paymentOptionsWorker.js', import.meta.url), 'utf8')
    .replace(/^import[^\n]+\n/gm, '');
  vm.runInNewContext(code, { performance, self, createLocalAnalysisReplica, WasmGame: Game, initWasm: async () => {} });
  const operations = [{ method: 'registerDefinition', args: ['definition'], failed: false }];
  const send = (token, value, epoch = 1) => {
    operations.push({ method: 'setState', args: [value, token % 2], failed: false });
    return self.onmessage({ data: { token, localReplay: { epoch, identityOrigin: { object: 1 },
      operations: structuredClone(operations) }, request: String(token) } });
  };
  await send(1, 'first');
  await send(2, 'second');
  assert.equal(games.length, 1);
  assert.deepEqual(registered, ['definition']);
  assert.equal(messages[1].token, 2);
  assert.equal(messages[1].result.state.value, 'second');
  assert.equal(messages[1].result.state.perspective, 0);
  assert.equal(messages[0].result.state.value, 'first');
  await send(3, 'third', 2);
  for (const message of messages) {
    for (const key of ['replayMs', 'computeOptionsMs', 'totalHandlerMs']) {
      const value = message.result.__payment_options_perf[key];
      assert.ok(Number.isFinite(value) && value >= 0, key);
    }
  }
  assert.equal(games.length, 2);
  assert.equal(games[0].freed, true);
  assert.deepEqual(registered, ['definition', 'definition']);
  assert.equal(messages[2].result.state.value, 'third');
});
