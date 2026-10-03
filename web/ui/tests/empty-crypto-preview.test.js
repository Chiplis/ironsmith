import test from 'node:test';
import assert from 'node:assert/strict';
import { canReuseEmptyCryptoPreview } from '../src/lib/preview-crypto-material.js';

const empty = () => ({ requirements: [], rngReveals: [], shuffleProofs: [],
  localOpenings: [], remoteOpenings: [], remotePrivateViewProofs: [] });

test('unchanged material-free submission can reuse its empty preview', () => {
  assert.equal(canReuseEmptyCryptoPreview(empty()), true);
});

for (const [field, material] of Object.entries({
  requirements: { type: 'public_open' },
  rngReveals: { combinedSeedHex: 'agreed random branch' },
  shuffleProofs: { inputDeck: 'new private epoch' },
  localOpenings: { card: 'replacement source', timing: 'pre' },
  remoteOpenings: { card: 'trigger source', timing: 'pre' },
  remotePrivateViewProofs: { viewer: 0 },
})) {
  test(`${field} requires a fresh preview even when all other collections are empty`, () => {
    assert.equal(canReuseEmptyCryptoPreview({ ...empty(), [field]: [material] }), false);
    assert.equal(canReuseEmptyCryptoPreview({ ...empty(), [field]: undefined }), false);
    assert.equal(canReuseEmptyCryptoPreview({ ...empty(), [field]: null }), false);
  });
}
