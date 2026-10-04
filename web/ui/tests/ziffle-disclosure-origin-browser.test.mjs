import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { findZiffleDisclosureOrigin } from '../src/lib/ziffle-disclosure-origin.js';
import { assertZiffleOpeningOriginMatchesMetadata } from '../src/lib/multiplayer-audit.js';
import { authorizationHarness } from './ziffle-reveal-authorization-harness.mjs';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('real worker departure preserves origin-bound final hand disclosure requirements', { timeout: 120000 }, async t => {
  const vite = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await vite.listen(); t.after(() => vite.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/disclosure-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Disclosure lifecycle</title>' }));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/disclosure-test`);
  const capture = await page.evaluate(async () => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const decoder = createSnapshotDecoder(), pending = new Map();
    const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' }); let id = 0;
    const ready = new Promise((resolve, reject) => {
      worker.onerror = error => reject(new Error(error.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'error') return reject(new Error(data.error.message));
        if (data.type === 'ready') return resolve();
        if (data.type !== 'result') return;
        const request = pending.get(data.id); if (!request) return; pending.delete(data.id);
        if (!data.ok) request.reject(new Error(data.error.message));
        else request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
      };
    });
    const call = (method, ...args) => new Promise((resolve, reject) => {
      pending.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
    });
    worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` }); await ready;
    try {
      await call('setPerspective', 1);
      await call('startMatch', { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1, format: 'normal',
        startingPlayer: 0, openingHandSize: 7, decks: [Array(60).fill('Island'), Array(60).fill('Mountain')],
        publicDecklists: [Array(60).fill('Island'), Array(60).fill('Mountain')],
        hiddenDeckManifests: [0, 1].map(owner => ({ owner, deckCount: 60, commitmentRoot: `ziffle:initial-${owner}`,
          slotCommitments: Array.from({ length: 60 }, (_, slot) => ({ slot, commitment: `ziffle:initial-${owner}:${slot}` })) })) });
      const before = await call('getHiddenCardState');
      const initialHand = before.objects.filter(object => object.hiddenCard?.owner === 1 && object.zone === 'hand');
      const state = await call('forfeitPlayer', 1);
      const checkpoint = await call('getHiddenCardState');
      const requirements = await call('endOfMatchDisclosureRequirements', 1);
      await call('setPerspective', 0);
      const remoteRequirements = await call('endOfMatchDisclosureRequirements', 1);
      return { state, checkpoint, initialHand, requirements, remoteRequirements };
    } finally { worker.terminate(); }
  });
  assert.ok(capture.state.game_over);
  assert.equal(capture.checkpoint.objects.filter(object => object.hiddenCard?.owner === 1).length, 0);
  assert.equal(capture.initialHand.length, 7);
  assert.equal(capture.requirements.length, 7);
  assert.deepEqual(capture.requirements.map(requirement => requirement.originSlot).sort((a, b) => a - b),
    capture.initialHand.map(object => object.hiddenCard.originSlot).sort((a, b) => a - b),
    'every departed hand card retains its original ciphertext position');
  const positions = [];
  for (const requirement of capture.requirements) {
    const positionCommitment = requirement.publicCommitment || requirement.public_commitment || requirement.commitment;
    const opening = { owner: 1, positionCommitment, position: Number(positionCommitment.split(':').at(-1)),
      originPosition: requirement.originSlot, originPositionCommitment: requirement.originCommitment };
    const found = findZiffleDisclosureOrigin({ opening, state: capture.state, requirements: capture.remoteRequirements });
    assert.ok(found, `locally retained final requirement binds ${positionCommitment}`);
    assertZiffleOpeningOriginMatchesMetadata(opening, found.metadata);
    assert.equal(findZiffleDisclosureOrigin({ opening, state: { players: [{ id: 1 }] }, requirements: capture.remoteRequirements }), null);
    positions.push(opening.originPosition);
  }
  const h = authorizationHarness({ checkpoint: capture.checkpoint, disclosureRequirements: capture.remoteRequirements, disclosureDue: true });
  assert.deepEqual([...(await h.visiblePositions(1, 'initial-1'))].sort((a, b) => a - b), positions.sort((a, b) => a - b));
  assert.equal((await h.visiblePositions(1, 'unrelated')).size, 0);
  const active = authorizationHarness({ checkpoint: capture.checkpoint, disclosureRequirements: capture.remoteRequirements });
  assert.equal((await active.visiblePositions(1, 'initial-1')).size, 0);
});
