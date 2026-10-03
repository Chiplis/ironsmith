import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

// Exercise the actual worker handler with a deterministic runtime boundary.
// The browser suite independently checks real WASM outputs and peer audits.
test('payment worker reuses definitions but replaces state for each request', async () => {
  const messages = [], games = [], registered = [];
  class Game {
    constructor() { games.push(this); }
    setDeferredPriorityAnalysis() {}
    setDeferredManaOptions() {}
    importSyncCheckpoint(checkpoint, perspective) { this.state = { ...checkpoint, perspective }; }
    getPaymentActivationOptions(request) { return { request, state: this.state }; }
    free() { this.freed = true; }
  }
  const self = { postMessage(message) { messages.push(message); } };
  const code = readFileSync(new URL('../src/workers/paymentOptionsWorker.js', import.meta.url), 'utf8')
    .replace(/^import[^\n]+\n/, '');
  vm.runInNewContext(code, { performance, self, WasmGame: Game, initWasm: async () => {},
    compileAndRegisterCardSources(game, sources) { registered.push(...sources); return {}; } });
  const send = (token, state, sources = [['route', 'definition']]) => self.onmessage({ data: {
    token, checkpoint: { perspective: token % 2, state }, sources, registrations: [], request: String(token),
  } });
  await send(1, 'first');
  await send(2, 'second');
  assert.equal(games.length, 1);
  assert.deepEqual(registered, ['definition']);
  assert.equal(messages[1].token, 2);
  assert.equal(messages[1].result.state.state, 'second');
  assert.equal(messages[1].result.state.perspective, 0);
  assert.equal(messages[0].result.state.state, 'first');
  await send(3, 'third', [['route', 'changed definition']]);
  const first = messages[0].result.__payment_options_perf;
  const reused = messages[1].result.__payment_options_perf;
  const changed = messages[2].result.__payment_options_perf;
  assert.equal(first.coldGame, true);
  assert.equal(first.rebuiltGame, true);
  assert.equal(first.sourceCount, 1);
  assert.equal(reused.coldGame, false);
  assert.equal(reused.registryKeyChanged, false);
  assert.equal(reused.rebuiltGame, false);
  assert.equal(reused.sourceRegistrationMs, 0);
  assert.equal(changed.registryKeyChanged, true);
  assert.equal(changed.rebuiltGame, true);
  for (const phases of [first, reused, changed]) {
    for (const key of ['initMs', 'registryKeyMs', 'createGameMs', 'disposeGameMs', 'constructorMs', 'deferredPriorityConfigMs', 'deferredManaConfigMs', 'sourceRegistrationMs', 'registrationReplayMs', 'importCheckpointMs', 'computeOptionsMs', 'totalHandlerMs']) {
      assert.ok(Number.isFinite(phases[key]) && phases[key] >= 0, key);
    }
  }
  assert.equal(games.length, 2);
  assert.equal(games[0].freed, true);
  assert.deepEqual(registered, ['definition', 'changed definition']);
  assert.equal(messages[2].result.state.state, 'third');
});
