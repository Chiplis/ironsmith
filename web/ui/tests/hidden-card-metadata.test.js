import test from 'node:test';
import assert from 'node:assert/strict';
import { hiddenCardMetadataForObjectFromCheckpoint, hiddenCardMetadataAtPositionFromCheckpoint } from '../src/lib/hidden-card-metadata.js';

test('projection returns only identity metadata and observes replacements on each request', () => {
  const hidden = { owner: 1, slot: 4, commitment: 'salted', publicSlot: 51,
    publicCommitment: 'ziffle:later:51', originSlot: 23, originCommitment: 'ziffle:initial:23' };
  const checkpoint = { objects: [{ id: 212, zone: 'hand', name: 'Private card', card: { secret: true }, hiddenCard: hidden }] };
  const expected = { objectId: 212, zone: 'hand', ...hidden };
  assert.deepEqual(hiddenCardMetadataForObjectFromCheckpoint(checkpoint, 212), expected);
  assert.deepEqual(hiddenCardMetadataAtPositionFromCheckpoint(checkpoint, 1, 51, hidden.publicCommitment), [expected]);
  assert.deepEqual(hiddenCardMetadataAtPositionFromCheckpoint(checkpoint, 0, 51, hidden.publicCommitment), []);
  assert.deepEqual(hiddenCardMetadataAtPositionFromCheckpoint(checkpoint, 1, 50, hidden.publicCommitment), []);
  assert.deepEqual(hiddenCardMetadataAtPositionFromCheckpoint(checkpoint, 1, 51, 'ziffle:other:51'), []);
  checkpoint.objects[0] = { ...checkpoint.objects[0], id: 213, zone: 'battlefield' };
  assert.equal(hiddenCardMetadataForObjectFromCheckpoint(checkpoint, 212), null);
  assert.equal(hiddenCardMetadataAtPositionFromCheckpoint(checkpoint, 1, 51, hidden.publicCommitment)[0].objectId, 213);
  assert.equal(hiddenCardMetadataForObjectFromCheckpoint(checkpoint, NaN), null);
});

test('snake-case metadata and duplicate position candidates are retained without guessing', () => {
  const checkpoint = { objects: [1, 2].map(id => ({ id, zone: 'exile', hidden_card: {
    owner: 0, slot: 3, commitment: 'old', public_slot: 7, public_commitment: 'current',
    origin_slot: 2, origin_commitment: 'origin',
  } })) };
  const candidates = hiddenCardMetadataAtPositionFromCheckpoint(checkpoint, 0, 7, 'current');
  assert.deepEqual(candidates.map(entry => entry.objectId), [1, 2]);
  assert.equal(candidates[0].originSlot, 2);
  assert.equal(candidates[0].originCommitment, 'origin');
});

test('worker dispatch reads native metadata and propagates native errors', async () => {
  const { readFileSync } = await import('node:fs');
  const source = readFileSync(new URL('../src/workers/wasmGameWorker.js', import.meta.url), 'utf8');
  const start = source.indexOf('    const fn = method === "replayTrustedMatch"');
  const end = source.indexOf('    if (typeof fn !== "function")', start);
  assert.ok(start >= 0 && end > start);
  const route = new Function('method', 'game', 'hiddenCardMetadataForObjectFromCheckpoint',
    'hiddenCardMetadataAtPositionFromCheckpoint', `${source.slice(start, end)}\nreturn fn;`);
  const select = (method, game) => route(method, game,
    hiddenCardMetadataForObjectFromCheckpoint, hiddenCardMetadataAtPositionFromCheckpoint);
  const checkpoint = {objects:[{id:8,zone:'hand',hiddenCard:{owner:0,slot:4,commitment:'current'}}]};
  const expected = hiddenCardMetadataForObjectFromCheckpoint(checkpoint, 8);
  const native = {
    getHiddenCardState() { assert.equal(this, native); return checkpoint; },
    getHiddenCardMetadata(id) { assert.equal(this, native); assert.equal(id, 8); return expected; },
    getHiddenCardMetadataAtPosition(owner,position,commitment) {
      assert.equal(this, native); assert.deepEqual([owner,position,commitment],[0,4,'current']); return [expected,expected];
    },
  };
  assert.deepEqual(select('getHiddenCardMetadata', native).call(native,8), expected);
  assert.equal(select('getHiddenCardMetadataAtPosition', native).call(native,0,4,'current').length,2);
  assert.equal(select('getHiddenCardState', native).call(native), checkpoint);
  native.getHiddenCardMetadata = () => {throw new Error('metadata failure');};
  assert.throws(() => select('getHiddenCardMetadata', native).call(native,8), /metadata failure/);
});
