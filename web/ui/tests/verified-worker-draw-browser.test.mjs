import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { publicCheckpointHash } from '../src/lib/multiplayer-audit.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function hiddenManifest(owner) {
  return {
    owner, deckCount: 60, commitmentRoot: `root-${owner}`, decklistHash: `deck-${owner}`,
    slotCommitments: Array.from({ length: 60 }, (_, slot) => ({ slot, commitment: `commitment-${owner}-${slot}` })),
  };
}

test('verified workers agree through the host second draw with asymmetric card caches', { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen();
  t.after(() => server.close());
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/worker-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Verified draw regression</title>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/worker-test`);
  const result = await page.evaluate(async ({ manifests }) => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    function createWorker() {
      const decoder = createSnapshotDecoder();
      const pending = new Map();
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
          if (!data.ok) { request.reject(new Error(data.error.message)); return; }
          request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
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
      // Lobby validation warms only the local deck. These two spells are
      // deliberately outside the initial worker registry.
      const names = ['Pillage', 'Cultivate'];
      const basics = ['Island', 'Forest'];
      const decklists = names.map((name, seat) => [...Array(4).fill(name), ...Array(56).fill(basics[seat])]);
      await Promise.all(workers.map((worker, seat) => worker.call('filterKnownCardNames', decklists[seat])));
      const cachedRoutes = await Promise.all(workers.map(worker => worker.call('getExternalCardRoutes')));
      const config = {
        playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1,
        // Six cards avoid an unrelated cleanup discard while passing turns.
        format: 'normal', startingPlayer: 0, openingHandSize: 6,
        decks: [[], []], sideboards: [[], []], commanders: [[], []],
        publicDecklists: decklists, hiddenDeckManifests: manifests,
      };
      await Promise.all(workers.map((worker, seat) => worker.call('setPerspective', seat)));
      let states = await Promise.all(workers.map(worker => worker.call('startMatch', config)));
      async function hydrateOwnHand(worker, seat) {
        const checkpoint = await worker.call('getHiddenCardState');
        for (const id of checkpoint.players[seat].hand) {
          const object = checkpoint.objects.find(object => object.id === id);
          if (object.name !== 'Hidden Card') continue;
          await worker.call('revealHiddenSlot', { ...object.hiddenCard, cardName: basics[seat] });
        }
        return worker.call('uiState');
      }
      states = await Promise.all(workers.map(hydrateOwnHand));
      const trace = [];
      const publicDecision = state => ({ turn: state.turn_number, phase: state.phase, step: state.step,
        active: state.active_player, priority: state.priority_player, kind: state.decision?.kind,
        player: state.decision?.player, hands: state.players.map(player => player.hand_size) });
      for (let step = 0; step < 160; step++) {
        const decisions = states.map(publicDecision);
        trace.push(decisions[0]);
        if (JSON.stringify(decisions[0]) !== JSON.stringify(decisions[1])) {
          throw new Error(`Peers disagreed: ${JSON.stringify({ decisions, trace })}`);
        }
        const action = states[0].decision?.actions?.find(candidate =>
          ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(candidate.action_ref?.kind));
        const kind = states[0].decision?.kind;
        if (!action && kind !== 'attackers' && kind !== 'blockers') {
          throw new Error(`Cannot advance: ${JSON.stringify(states[0].decision)}`);
        }
        const reachedHostDraw = states.every(state => state.turn_number >= 3 && state.players[0].hand_size === 7);
        const command = action ? { type: 'priority_action', action_ref: action.action_ref }
          : { type: kind === 'attackers' ? 'declare_attackers' : 'declare_blockers', declarations: [] };
        await Promise.all(workers.map(worker => worker.call('dispatch', command)));
        states = await Promise.all(workers.map(hydrateOwnHand));
        if (reachedHostDraw) {
          const finalCheckpoints = await Promise.all(workers.map(worker => worker.call('getHiddenCardState')));
          const opponentViews = await Promise.all(workers.map(async (worker, seat) => {
            await worker.call('setPerspective', 1 - seat);
            const view = await worker.call('uiState');
            await worker.call('setPerspective', seat);
            return view;
          }));
          const audits = await Promise.all(workers.map(worker => worker.call('exportPublicAuditCheckpoint')));
          return { cachedRoutes, trace, final: states.map(publicDecision), finalCheckpoints, opponentViews, audits };
        }
      }
      throw new Error(`Did not reach Alice's second draw: ${JSON.stringify(trace)}`);
    } finally { workers.forEach(worker => worker.terminate()); }
  }, { manifests: [hiddenManifest(0), hiddenManifest(1)] });
  assert.equal(result.cachedRoutes[0].includes('pillage'), true);
  assert.equal(result.cachedRoutes[1].includes('pillage'), false);
  assert.equal(result.cachedRoutes[0].includes('cultivate'), false);
  assert.equal(result.cachedRoutes[1].includes('cultivate'), true);
  assert.deepEqual(result.final[0], result.final[1]);
  assert.equal(result.final[0].turn, 3);
  assert.deepEqual(result.final[0].hands, [7, 7]);
  assert.ok(result.trace.some(state => state.turn === 2 && /draw/i.test(state.step) && state.hands[1] === 7));
  assert.ok(result.trace.some(state => state.turn === 3 && /draw/i.test(state.step) && state.hands[0] === 7));
  // Compare the same canonical state peers sign, excluding worker timing data.
  assert.equal(await publicCheckpointHash(result.audits[0]), await publicCheckpointHash(result.audits[1]),
    'public audit state agrees after the command following Alice draw');
  for (let seat = 0; seat < 2; seat++) {
    const own = result.finalCheckpoints[seat];
    const other = result.finalCheckpoints[1 - seat];
    const hiddenIds = own.objects.filter(object => object.hiddenCard?.owner === seat
      && ['hand', 'library'].includes(object.zone)).map(object => object.id);
    for (const id of hiddenIds) {
      assert.equal(other.objects.find(object => object.id === id).name, 'Hidden Card');
    }
    assert.equal(result.opponentViews[seat].players[seat].can_view_hand, false);
    assert.deepEqual(result.opponentViews[seat].players[seat].hand_cards, []);
    for (const id of own.players[seat].hand) {
      assert.equal(own.objects.find(object => object.id === id).name, ['Island', 'Forest'][seat]);
    }
  }
});
