import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { createLocalAnalysisReplica } from '../src/lib/local-analysis-replay.js';

// Rejected diagnostic sources are ordinary registry results, while an invalid
// checkpoint must still fail the operation. Exercise both auxiliary workers.
for (const worker of ['paymentOptionsWorker', 'targetPreviewWorker']) {
  for (const missingDefinition of [false, true]) {
    test(`${worker} ${missingDefinition ? 'reports divergent definition failures' : 'continues past a rejected diagnostic source'}`, async () => {
      const messages = [], registered = [];
      class Game {
        points = new Map(); nextHandle = 0;
        createRuntimeSavepoint() { const h = ++this.nextHandle; this.points.set(h, {}); return h; }
        exchangeRuntimeSavepoint() {}
        copyRuntimeSavepoint() {}
        releaseRuntimeSavepoint(h) { return this.points.delete(h); }
        setPerspective() {}
        free() {}
        registerSource(source) { registered.push(source); return { failed: source === 'rejected' ? ['unsupported'] : [] }; }
        setDeferredPriorityAnalysis() {}
        setDeferredManaOptions() {}
        initializeRuntimeIdentityOrigin() {}
        requireDefinition(missing) { if (missing) throw new Error('missing required definition'); }
        getPaymentActivationOptions() { return { options: ['pay'] }; }
        dispatch() { return { decision: { kind: 'targets', requirements: ['target'] } }; }
      }
      const self = { postMessage: message => messages.push(message) };
      const code = readFileSync(new URL(`../src/workers/${worker}.js`, import.meta.url), 'utf8')
        .replace(/^import[^\n]+\n/gm, '');
      vm.runInNewContext(code, { self, performance, createLocalAnalysisReplica, WasmGame: Game, initWasm: async () => {},
        castingMethodChoiceForAction: () => null, setTimeout: fn => fn(),
        compileAndRegisterCardSources: (_, sources) => {
          registered.push(...sources);
          return { failed: sources.includes('rejected') ? [{ error: 'unsupported mechanics' }] : [] };
        } });
      await self.onmessage({ data: { token: 1, id: 1, sources: [['bad', 'rejected'], ['good', 'supported']],
        localReplay: { epoch: 1, identityOrigin: { object: 1 }, operations: [
          { method: 'registerSource', args: ['rejected'], failed: false },
          { method: 'registerSource', args: ['supported'], failed: false },
          { method: 'requireDefinition', args: [missingDefinition], failed: false },
        ] }, perspective: 1,
        request: 'payment', actions: [{ index: 1, action_ref: { kind: 'cast_spell' } }] } });
      assert.deepEqual(registered, ['rejected', 'supported']);
      assert.equal(messages.length, 1);
      if (missingDefinition) assert.match(messages[0].error, /missing required definition/);
      else {
        assert.equal(messages[0].error, undefined);
        assert.ok(messages[0].result);
      }
    });
  }
}
