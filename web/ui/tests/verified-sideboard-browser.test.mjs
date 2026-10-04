import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { buildPrivateDeckManifest, publicCheckpointHash } from '../src/lib/multiplayer-audit.js';
import { buildZiffleRuntimeManifest } from '../src/lib/ziffle-runtime-manifest.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

for (const explicitSideboards of [false, true]) {
  test(`verified workers retain every committed ${explicitSideboards ? 'explicit' : 'hidden'} sideboard slot in a 61-card constructed match`, { timeout: 120000 }, async t => {
    const decks = [Array(61).fill('Island'), Array(61).fill('Plains')];
    const sideboards = [Array(15).fill('Forest'), Array(15).fill('Mountain')];
    const originals = await Promise.all(decks.map((deck, owner) => buildPrivateDeckManifest({
      matchId: 'verified-sideboard-regression', owner, deck, sideboard: sideboards[owner],
    })));
    const manifests = originals.map((manifest, owner) => buildZiffleRuntimeManifest(manifest, {
      deckCount: 61, deckHash: `verified-deck-${owner}`,
    }));
    const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
      server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await server.listen();
    t.after(() => server.close());
    const browser = await chromium.launch();
    t.after(() => browser.close());
    const page = await browser.newPage();
    await page.route('**/sideboard-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Hidden sideboard regression</title>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/sideboard-test`);
    const result = await page.evaluate(async ({ manifests, decks, sideboards, explicitSideboards }) => {
      const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
      function createWorker() {
        const decoder = createSnapshotDecoder(), pending = new Map();
        const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
        let id = 0;
        const ready = new Promise((resolve, reject) => {
          worker.onerror = error => reject(new Error(error.message));
          worker.onmessage = ({ data }) => {
            if (data.type === 'error') { reject(new Error(data.error.message)); return; }
            if (data.type === 'ready') { resolve(); return; }
            if (data.type !== 'result') return;
            const request = pending.get(data.id);
            if (!request) return;
            pending.delete(data.id);
            if (!data.ok) request.reject(new Error(data.error.message));
            else request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
          };
        });
        const call = (method, ...args) => new Promise((resolve, reject) => {
          pending.set(++id, { resolve, reject });
          worker.postMessage({ type: 'call', id, method, args });
        });
        worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
        return { ready, call, terminate: () => worker.terminate() };
      }
      const workers = [createWorker(), createWorker()];
      try {
        await Promise.all(workers.map(worker => worker.ready));
        const config = {
          playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1, format: 'normal',
          startingPlayer: 0, openingHandSize: 7,
          decks: [[], []], sideboards: explicitSideboards ? sideboards : [[], []], commanders: [[], []],
          publicDecklists: decks.map((deck, owner) => [...deck, ...sideboards[owner]]),
          hiddenDeckManifests: manifests,
        };
        const errors = [];
        for (const mutation of ['missing', 'duplicate', 'empty']) {
          const malformed = structuredClone(config);
          const slots = malformed.hiddenDeckManifests[0].slotCommitments;
          if (mutation === 'missing') slots.pop();
          else if (mutation === 'duplicate') slots.at(-1).slot = slots.at(-2).slot;
          else slots.at(-1).commitment = '';
          try {
            await workers[0].call('startMatch', malformed);
            errors.push(null);
          } catch (error) { errors.push(error.message); }
        }
        await Promise.all(workers.map((worker, seat) => worker.call('setPerspective', seat)));
        await Promise.all(workers.map(worker => worker.call('startMatch', config)));
        const before = await Promise.all(workers.map(worker => worker.call('getHiddenCardState')));
        const beforeViews = await Promise.all(workers.map(worker => worker.call('uiState')));
        // Owners hydrate their committed sideboards; opponent UI views keep
        // those identities private.
        for (let seat = 0; seat < workers.length; seat++) {
          for (const id of before[seat].players[seat].sideboard) {
            const object = before[seat].objects.find(object => object.id === id);
            await workers[seat].call('revealHiddenSlot', { ...object.hiddenCard, cardName: sideboards[seat][0] });
          }
        }
        const after = await Promise.all(workers.map(worker => worker.call('getHiddenCardState')));
        const opponentViews = await Promise.all(workers.map(async (worker, seat) => {
          await worker.call('setPerspective', 1 - seat);
          const view = await worker.call('uiState');
          await worker.call('setPerspective', seat);
          return view;
        }));
        const audits = await Promise.all(workers.map(worker => worker.call('exportPublicAuditCheckpoint')));
        return { errors, before, beforeViews, after, opponentViews, audits };
      } finally { workers.forEach(worker => worker.terminate()); }
    }, { manifests, decks, sideboards, explicitSideboards });
    assert.match(result.errors[0], /must commit every main-deck and sideboard slot/);
    assert.match(result.errors[1], /empty or duplicate slot commitment/);
    assert.match(result.errors[2], /empty or duplicate slot commitment/);
    for (let seat = 0; seat < 2; seat++) {
      assert.equal(manifests[seat].slotCommitments.length, 76);
      assert.deepEqual(manifests[seat].slotCommitments.slice(61), originals[seat].slotCommitments.slice(61));
      for (const view of result.beforeViews) {
        assert.equal(view.players[seat].library_size, 54);
        assert.equal(view.players[seat].hand_size, 7);
      }
      for (const metadata of result.before) {
        assert.equal(metadata.players[seat].sideboard.length, 15);
      }
      const ids = result.before[seat].players[seat].sideboard;
      assert.deepEqual(ids.map(id => result.before[seat].objects.find(object => object.id === id).hiddenCard.slot),
        Array.from({ length: 15 }, (_, index) => 61 + index));
      assert.deepEqual(result.opponentViews[seat].players[seat].sideboard_cards, [],
        "opponent UI exposes no private sideboard identities");
      for (const id of ids) {
        const original = result.before[seat].objects.find(object => object.id === id);
        assert.equal(original.zone, 'outside_game');
        assert.equal(original.name, explicitSideboards ? sideboards[seat][0] : 'Hidden Card');
        assert.ok(original.hiddenCard.slot >= 61 && original.hiddenCard.slot < 76);
        assert.equal(original.hiddenCard.commitment, originals[seat].slotCommitments[original.hiddenCard.slot].commitment);
        assert.equal(result.after[seat].objects.find(object => object.id === id).name, sideboards[seat][0]);
        assert.equal(result.after[1 - seat].objects.find(object => object.id === id).name, explicitSideboards ? sideboards[seat][0] : 'Hidden Card');
      }
    }
    assert.equal(await publicCheckpointHash(result.audits[0]), await publicCheckpointHash(result.audits[1]));
  });
}
