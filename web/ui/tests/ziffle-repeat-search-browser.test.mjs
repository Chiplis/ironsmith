import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const source = readFileSync(new URL('../src/hooks/peer-lobby/crypto-resync.js', import.meta.url), 'utf8');
const start = source.indexOf('  function cryptoRequirementReplayKey(');
const end = source.indexOf('\n  function shuffleProofReplayKey(', start);
assert.ok(start >= 0 && end > start, 'extract the production replay filter');
const replayFilterSource = source.slice(start, end);

// This exercises real engine decisions and the production requirement filter.
// Controlled commitment metadata stands in for verified ciphertexts; this is
// deliberately not a proof-verification or multiplayer-network test.
test('a second fetch reveals a resealed library again and keeps its names private', { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  const pageErrors = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  await page.route('**/repeat-search-test', route => route.fulfill({
    contentType: 'text/html', body: '<title>Repeated library search regression</title>',
  }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/repeat-search-test`);
  const results = await page.evaluate(async replayFilterSource => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    function createWorker() {
      const decoder = createSnapshotDecoder(), pending = new Map(), analyses = new Map(), analysisWaiters = new Map();
      const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
      let requestId = 0;
      const ready = new Promise((resolve, reject) => {
        worker.onerror = error => reject(new Error(error.message));
        worker.onmessage = ({ data }) => {
          if (data.type === 'error') return reject(new Error(data.error.message));
          if (data.type === 'ready') return resolve();
          if (data.type === 'priorityAnalysis') {
            if (data.decision?.analysis_complete !== true) return;
            analyses.set(data.revision, data.decision);
            analysisWaiters.get(data.revision)?.(data.decision);
            return;
          }
          if (data.type !== 'result') return;
          const request = pending.get(data.id); if (!request) return;
          pending.delete(data.id);
          if (!data.ok) request.reject(new Error(data.error.message));
          else request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
        };
      });
      const call = (method, ...args) => new Promise((resolve, reject) => {
        pending.set(++requestId, { resolve, reject });
        worker.postMessage({ type: 'call', id: requestId, method, args });
      }).then(async value => {
        if (value?.decision?.analysis_complete !== false) return value;
        const revision = value.__priority_revision;
        const decision = analyses.get(revision) || await new Promise(resolve => analysisWaiters.set(revision, resolve));
        analysisWaiters.delete(revision);
        return { ...value, decision };
      });
      worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
      return { ready, call, stop: () => worker.terminate() };
    }
    const owner = createWorker(), opponent = createWorker();
    const call = owner.call;
    const setupCall = async (method, ...args) => {
      const result = await call(method, ...args);
      await opponent.call(method, ...args);
      return result;
    };
    const deck = Array.from({ length: 60 }, (_, slot) => slot % 2 ? 'Swamp' : 'Island');
    for (const slot of [2, 8, 14, 20]) deck[slot] = 'Emperor of Bones';
    const privateOpenings = requirements => requirements.filter(value => value.type === 'private_open');
    const hydrate = async (requirements, engineCall = call) => {
      const reveals = privateOpenings(requirements).map(requirement => ({
        owner: requirement.owner, objectId: requirement.objectId,
        position: requirement.publicSlot ?? requirement.slot,
        originalSlot: requirement.slot, cardName: deck[requirement.slot],
        positionCommitment: requirement.publicCommitment ?? requirement.commitment,
        commitment: Number(requirement.slot + 1).toString(16).padStart(64, '0'),
      }));
      if (reveals.length) await engineCall('revealHiddenPositions', { reveals, recomputeDecision: true });
    };
    const select = candidate => ({ type: 'select_objects', object_ids: [candidate.id],
      object_hidden_refs: [{ owner: candidate.hidden_ref.owner, zone: candidate.hidden_ref.zone,
        public_slot: candidate.hidden_ref.public_slot, public_commitment: candidate.hidden_ref.public_commitment }] });
    const captures = [];
    try {
      await Promise.all([owner.ready, opponent.ready]);
      for (const legacy of [true, false]) {
        await call('setPerspective', 0);
        await opponent.call('setPerspective', 1);
        let state = await setupCall('startMatch', {
          playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1, format: 'normal',
          startingPlayer: 0, openingHandSize: 0, decks: [[], []],
          publicDecklists: [deck, Array(60).fill('Mountain')],
          hiddenDeckManifests: [0, 1].map(owner => ({ owner, deckCount: 60, commitmentRoot: `ziffle:genesis-${owner}`,
            slotCommitments: Array.from({ length: 60 }, (_, slot) => ({ slot, commitment: `ziffle:genesis-${owner}:${slot}` })) })),
        });
        const marsh = await setupCall('addCardToZone', 0, 'Marsh Flats', 'battlefield', true);
        const delta = await setupCall('addCardToZone', 0, 'Polluted Delta', 'battlefield', true);
        state = await call('uiState');
        const history = new Map();
        const canonical = value => {
          const identity = { ...value };
          // Reproduce the old key without modifying shared production source.
          if (legacy) for (const key of ['publicSlot', 'publicCommitment', 'originSlot', 'originCommitment']) delete identity[key];
          return JSON.stringify(identity);
        };
        const fresh = new Function('canonicalMultiplayerPayload', 'normalizeShuffleOrder', 'actionCryptoRequirementsRef',
          `${replayFilterSource}\nreturn freshCryptoRequirementsForSequence;`)(canonical, value => value || [], { current: history });
        const reachSearch = async (fetchId, sequence) => {
          let activated = false;
          for (let step = 0; step < 80; step++) {
            const actions = state.decision?.actions || [];
            let action = !activated && actions.find(value => value.action_ref?.kind === 'activate_ability'
              && Number(value.action_ref.source) === Number(fetchId));
            if (action) activated = true;
            action ||= actions.find(value => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(value.action_ref?.kind));
            if (!action) throw new Error(`Cannot reach fetch search: ${JSON.stringify(state.decision)}`);
            const command = { type: 'priority_action', action_ref: action.action_ref };
            const requirements = await call('previewCryptoRequirements', command);
            if (requirements.some(value => value.type === 'private_view_window')) {
              const retained = fresh(sequence, requirements);
              // The opponent receives only the action, with no private openings.
              await opponent.call('dispatch', command);
              await hydrate(retained);
              state = await call('dispatch', command);
              return { requirements, retained, state };
            }
            state = await setupCall('dispatch', command);
          }
          throw new Error('Fetch did not reach a library search');
        };
        const first = await reachSearch(marsh, 50);
        // The live protocol remembers finalized, hydrated engine requirements.
        history.set(50, state.crypto_requirements);
        const swamp = state.decision.candidates.find(candidate => candidate.name === 'Swamp' && candidate.legal);
        if (!swamp) throw new Error(`First fetch has no Swamp: ${JSON.stringify(state.decision)}`);
        // The selected land becomes public on both clients; other library
        // openings stay solely with the searching player.
        await hydrate(first.retained.filter(requirement => requirement.objectId === swamp.id), opponent.call);
        state = await setupCall('dispatch', select(swamp));
        const afterFirst = await call('getHiddenCardState');
        const afterOrder = afterFirst.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library')
          .sort((left, right) => (right.hiddenCard.publicSlot ?? right.hiddenCard.slot) - (left.hiddenCard.publicSlot ?? left.hiddenCard.slot)).map(object => object.id);
        await setupCall('applyVerifiedHiddenLibraryShuffle', { owner: 0, deckHash: 'after-first-fetch', afterOrder });
        state = await call('uiState');
        const second = await reachSearch(delta, 99);
        const candidates = state.decision.candidates;
        const capture = { legacy, firstPrivateCount: privateOpenings(first.retained).length,
          secondPrivateCount: privateOpenings(second.requirements).length,
          retainedPrivateCount: privateOpenings(second.retained).length,
          names: candidates.map(candidate => candidate.name), legal: candidates.map(candidate => candidate.legal),
          firstFetched: afterFirst.objects.filter(object => object.zone === 'battlefield').map(object => object.name) };
        const opponentState = await opponent.call('uiState');
        capture.opponentNames = [...(opponentState.decision?.candidates || []), ...(opponentState.viewed_cards?.cards || [])].map(card => card.name);
        const opponentCheckpoint = await opponent.call('getHiddenCardState');
        capture.opponentLibraryNames = opponentCheckpoint.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library').map(object => object.name);
        if (legacy) {
          const nonland = state.decision.candidates.find(candidate => deck[candidate.hidden_ref.slot] === 'Emperor of Bones');
          if (!nonland) throw new Error('Legacy search must contain the hidden Emperor');
          const opening = second.requirements.find(requirement => requirement.objectId === nonland.id);
          await hydrate([opening]);
          const revealed = await call('uiState');
          capture.afterSingleRevealCount = revealed.decision.candidates.length;
          try { await call('dispatch', select(nonland)); capture.error = ''; }
          catch (error) { capture.error = error.message; }
        } else {
          const island = state.decision.candidates.find(candidate => candidate.name === 'Island' && candidate.legal);
          if (!island) throw new Error('Second fetch has no revealed Island');
          await hydrate(second.retained.filter(requirement => requirement.objectId === island.id), opponent.call);
          state = await setupCall('dispatch', select(island));
          capture.finalDecision = state.decision.kind;
          capture.finalBattlefield = state.players[0].battlefield.map(card => card.name);
        }
        captures.push(capture);
      }
      return captures;
    } finally { owner.stop(); opponent.stop(); }
  }, replayFilterSource);
  assert.deepEqual(pageErrors, []);
  const [legacy, fixed] = results;
  assert.ok(legacy.firstPrivateCount >= 50, 'the first search privately opens the library');
  assert.ok(legacy.firstFetched.includes('Swamp'), 'Marsh Flats finishes fetching a land');
  assert.equal(legacy.retainedPrivateCount, 0, 'the old identity suppresses every repeated private opening');
  assert.ok(legacy.names.length >= 50);
  assert.ok(legacy.names.every(name => name === 'Hidden Card'), 'the old filter reproduces the hidden-card screen');
  assert.ok(legacy.legal.every(Boolean), 'concealed nonlands remain selectable before hydration');
  assert.equal(legacy.afterSingleRevealCount, legacy.names.length - 1, 'revealing the chosen Emperor removes exactly one choice');
  assert.match(legacy.error, /hidden object reference is not legal/, 'selecting that stale choice reproduces the reported failure');
  assert.equal(fixed.retainedPrivateCount, fixed.secondPrivateCount, 'the new ciphertext opens every card again');
  assert.ok(fixed.retainedPrivateCount >= 50);
  assert.ok(fixed.names.length > 0);
  assert.ok(fixed.names.every(name => ['Island', 'Swamp'].includes(name)), 'only legal, named fetch targets are offered');
  assert.ok(fixed.legal.every(Boolean));
  assert.ok(fixed.opponentLibraryNames.length >= 50);
  assert.ok(fixed.opponentLibraryNames.every(name => name === 'Hidden Card'), 'the opponent engine never hydrates private library cards');
  assert.ok(fixed.opponentNames.every(name => /^hidden card$/i.test(name)), 'private hydration does not disclose names to the opponent');
  assert.equal(fixed.finalDecision, 'priority', 'Polluted Delta completes its search and shuffle');
  assert.ok(fixed.finalBattlefield.includes('Swamp') && fixed.finalBattlefield.includes('Island'));
});
