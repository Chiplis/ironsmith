import test from 'node:test';
import assert from 'node:assert/strict';
import { shouldShareAnalysisRuntime, preferredVerifierWorkerCount } from '../src/lib/analysis-memory-policy.js';

test('iOS avoids duplicate analysis heaps, including desktop-mode iPadOS', () => {
  assert.equal(shouldShareAnalysisRuntime({ userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)' }), true);
  assert.equal(shouldShareAnalysisRuntime({ userAgent: 'Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X)' }), true);
  assert.equal(shouldShareAnalysisRuntime({ userAgent: 'Mozilla/5.0 (Macintosh)', platform: 'MacIntel', maxTouchPoints: 5 }), true);
  assert.equal(shouldShareAnalysisRuntime({ platform: 'MacIntel', maxTouchPoints: 0 }), false);
  assert.equal(shouldShareAnalysisRuntime({ userAgent: 'Chrome', platform: 'Win32', maxTouchPoints: 10 }), false);
  assert.equal(shouldShareAnalysisRuntime(undefined), false);
});

test('iOS uses one verifier heap even with many cores or a configured pool', () => {
  assert.equal(preferredVerifierWorkerCount({ userAgent: 'iPhone', hardwareConcurrency: 8 }, 8), 1);
  assert.equal(preferredVerifierWorkerCount({ platform: 'MacIntel', maxTouchPoints: 5, hardwareConcurrency: 16 }, 8), 1);
  assert.equal(preferredVerifierWorkerCount({ platform: 'MacIntel', hardwareConcurrency: 8 }), 4);
  assert.equal(preferredVerifierWorkerCount({ hardwareConcurrency: 8 }, 2), 2);
});
