import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

for (const scenario of ['land', 'cast']) test(`real shuffled reveal retains its committed copy through ${scenario} zone movement`, { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  const pageErrors = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  await page.route('**/opening-zone-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root"></div>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/opening-zone-test`);
  const result = await page.evaluate(async ({ verifierUrl, scenario }) => {
    const cardName = scenario === 'cast' ? 'Goblin Guide' : 'Barbarian Ring';
    const { mountOpeningServices } = await import('/tests/ziffle-public-opening-zone-change-harness.js');
    const { buildPrivateDeckManifest, publicDeckManifest, publicCheckpointHash } = await import('/src/lib/multiplayer-audit.js');
    const { buildZiffleRuntimeManifest } = await import('/src/lib/ziffle-runtime-manifest.js');
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const api = await import(verifierUrl);
    await api.default();
    function createWorkerSession() {
      const decoder = createSnapshotDecoder(), pending = new Map(), analyses = new Map(), analysisWaiters = new Map();
      const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
      let requestId = 0;
      const ready = new Promise((resolve, reject) => {
        worker.onerror = e => reject(new Error(e.message));
        worker.onmessage = ({ data }) => {
          if (data.type === 'error') return reject(new Error(data.error.message));
          if (data.type === 'ready') return resolve();
          if (data.type === 'priorityAnalysis') { if (data.decision?.analysis_complete !== true) return; analyses.set(data.revision, data.decision); analysisWaiters.get(data.revision)?.(data.decision); return; }
          if (data.type !== 'result') return;
          const request = pending.get(data.id); if (!request) return;
          pending.delete(data.id);
          if (!data.ok) request.reject(new Error(data.error.message));
          else request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
        };
      });
      const call = (method, ...args) => new Promise((resolve, reject) => {
        pending.set(++requestId, { resolve, reject }); worker.postMessage({ type: 'call', id: requestId, method, args });
      }).then(async value => {
        if (value?.decision?.analysis_complete !== false) return value;
        const revision = value.__priority_revision;
        const decision = analyses.get(revision) || await new Promise(resolve => analysisWaiters.set(revision, resolve));
        analysisWaiters.delete(revision); return { ...value, decision };
      });
      worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
      return { worker, call, ready };
    }
    const { worker, call: ownerCall, ready } = createWorkerSession();
    const setupCommands = [];
    const setupMethods = new Set(['startMatch', 'dispatch', 'applyVerifiedHiddenLibraryShuffle',
      'revealHiddenPosition', 'drawCard', 'addCardToZone']);
    let recordingSetup = true;
    const call = async (method, ...args) => {
      const result = await ownerCall(method, ...args);
      if (recordingSetup && setupMethods.has(method)) setupCommands.push([method, structuredClone(args)]);
      return result;
    };
    let peer;
    let reactRoot;
    try {
      await ready;
      const deck = Array(61).fill('Mountain');
      for (const slot of [2, 3, 4]) deck[slot] = cardName;
      const manifests = await Promise.all([0, 1].map(owner => buildPrivateDeckManifest({ matchId: 'zone-opening', owner, deck })));
      function buildCeremony(deckCount, context) {
        const identities = ['11', '22'].map(byte => api.ziffleKeygen({ deckCount, context, entropyHex: byte.repeat(32) }));
        const keys = identities.map((identity, player) => ({ player, publicKeyHex: identity.publicKeyHex, ownershipProofHex: identity.ownershipProofHex }));
        const steps = [];
        for (let shuffler = 0; shuffler < 2; shuffler++) {
          const step = api.ziffleBuildShuffleStep({ deckCount, context, keys, steps, shuffler, entropyHex: (context === 'zone-opening' ? ['33', '44'] : ['77', '88'])[shuffler].repeat(32) });
          steps.push({ shuffler, deckHex: step.deckHex, proofHex: step.proofHex });
        }
        const verified = api.ziffleVerifyShuffle({ deckCount, context, keys, steps });
        const positions = Array.from({ length: deckCount }, (_, index) => index);
        const tokens = identities.flatMap((identity, index) => api.ziffleBuildRevealTokens({ deckCount, context, keys, steps,
          ...identity, cardPositions: positions, entropyHex: ['55', '66'][index].repeat(32) }));
        const reveals = api.ziffleRevealCards({ deckCount, context, keys, steps, cardPositions: positions, tokens });
        return { owner: 1, deckCount, context, keys, steps, deckHash: verified.deckHash, tokens, reveals };
      }
      const genesis = buildCeremony(61, 'zone-opening');
      const originPosition = genesis.reveals.find(reveal => reveal.originalSlot === 4).cardPosition;

      await call('setPerspective', 1);
      let state = await call('startMatch', { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1, format: 'normal',
        startingPlayer: 1, openingHandSize: 0, decks: [[], []], publicDecklists: [deck, deck], hiddenDeckManifests: manifests.map((manifest, owner) => buildZiffleRuntimeManifest(manifest, { ...genesis, owner })) });
      for (let i = 0; i < 30 && state.phase !== 'first main phase'; i++) {
        const action = state.decision?.actions?.find(a => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(a.action_ref?.kind));
        if (!action) throw new Error(`Unexpected pregame ${JSON.stringify(state.decision)}`);
        state = await call('dispatch', { type: 'priority_action', action_ref: action.action_ref });
      }
      if (state.phase !== 'first main phase') throw new Error('Main phase missing');
      const beforeCheckpoint = await call('getHiddenCardState');
      const beforeOrder = beforeCheckpoint.objects.filter(object => object.hiddenCard?.owner === 1 && object.zone === 'library')
        .sort((left, right) => left.hiddenCard.slot - right.hiddenCard.slot).map(object => object.id);
      const originalFourId = beforeCheckpoint.objects.find(o => o.hiddenCard?.owner === 1 && o.hiddenCard?.slot === originPosition)?.id;
      const sourceIndex = beforeOrder.indexOf(originalFourId);
      if (sourceIndex < 0) throw new Error('Original slot4 not in library');
      if (sourceIndex === 2) [beforeOrder[2], beforeOrder[3]] = [beforeOrder[3], beforeOrder[2]];
      await call('applyVerifiedHiddenLibraryShuffle', { owner: 1, deckHash: 'prior-engine-order', afterOrder: beforeOrder });
      const current = buildCeremony(beforeOrder.length, 'zone-opening:action:1:shuffle');
      const { deckCount, context, keys, steps, tokens, reveals } = current;
      const position = reveals.find(reveal => reveal.originalSlot === 2).cardPosition;
      // The engine's object shuffle and the encrypted permutation are independent.
      const afterOrder = [...beforeOrder.slice(7), ...beforeOrder.slice(0, 7)];
      const destinationIndex = afterOrder.indexOf(originalFourId);
      [afterOrder[position], afterOrder[destinationIndex]] = [afterOrder[destinationIndex], afterOrder[position]];
      const ceremony = { owner: 1, deckCount, context, keys, steps, deckHash: current.deckHash,
        authenticatedOrder: true, beforeOrder, afterOrder };
      if (!ceremony.deckHash) throw new Error('Missing deck hash');
      await call('applyVerifiedHiddenLibraryShuffle', { owner: 1, deckHash: ceremony.deckHash, afterOrder });
      const secret = manifests[1].slotSecrets.find(entry => entry.slot === 4);
      const positionCommitment = `ziffle:${ceremony.deckHash}:${position}`;
      await call('revealHiddenPosition', { owner: 1, objectId: afterOrder[position], position,
        originalSlot: 4, cardName: secret.card, positionCommitment, commitment: secret.commitment });
      for (let index = deckCount - 1; index >= position; index--) await call('drawCard', 1);
      if (scenario === 'cast') await call('addCardToZone', 1, 'Mountain', 'battlefield', true);
      state = await call('uiState');
      const handRing = state.players[1].hand_cards.find(card => card.name === cardName);
      if (!handRing) throw new Error('Draw did not produce named Ring');
      const game = new Proxy({}, { get: (_, method) => method.startsWith('ziffle')
        ? async input => api[method](input)
        : (...args) => call(method, ...args) });
      const refs = { gameRef: { current: game }, stateRef: { current: state },
        multiplayerRef: { current: { localPlayerIndex: 1, players: manifests.map((manifest, index) => ({ index, deckAuditManifest: publicDeckManifest(manifest) })) } },
        verifiedAuditOpeningsRef: { current: new Set() },
        matchStartPayloadRef: { current: { auditMatchId: 'zone-opening', deckAuditManifests: manifests.map(publicDeckManifest), players: [{ index: 0 }, { index: 1 }] } },
        privateDeckManifestsRef: { current: new Map([['zone-opening:1', manifests[1]]]) },
        liveZiffleCeremoniesRef: { current: new Map([[1, ceremony]]) },
        localZiffleCeremonyLookupRef: { current: new Map([['initial', genesis]]) },
        setState() {}, setStatus() {}, setMultiplayer() {} };
      const base = new Proxy(refs, { get: (object, key) => object[key] ||= { current: new Map() } });
      const services = { current: { collectZiffleRevealTokens: async (selected, p) => (selected.context === genesis.context ? genesis.tokens : tokens).filter(token => token.cardPosition === p),
        collectZiffleRevealTokensBatch: async (selected, ps) => (selected.context === genesis.context ? genesis.tokens : tokens).filter(token => ps.includes(token.cardPosition)),
        commandObjectHiddenRefs: () => [], commandObjectStableIds: () => [],
        currentObjectIdForHiddenRef: async () => null, currentObjectIdForStableId: async () => null } };
      reactRoot = mountOpeningServices(base, services);
      const action = state.decision.actions.find(a => a.action_ref?.kind === (scenario === 'cast' ? 'cast_spell' : 'play_land') && Number(a.object_id) === handRing.id);
      if (!action) throw new Error('Ring not playable');
      const command = { type: 'priority_action', action_ref: action.action_ref, object_id: handRing.id };
      const requirements = await call('previewCryptoRequirements', command);
      const preOpenings = await services.current.buildLocalOpeningsForCommand(command, requirements);
      if (scenario === 'cast') {
        peer = createWorkerSession(); await peer.ready;
        // Reproduce the fixture's commands, including the card opening that is
        // public when announced, instead of transferring an engine snapshot.
        await peer.call('setPerspective', 0);
        for (const [method, args] of setupCommands) await peer.call(method, ...args);
      }
      recordingSetup = false;
      state = await call('dispatch', command); refs.stateRef.current = state;
      const afterCheckpoint = await call('getHiddenCardState');
      const fieldRing = afterCheckpoint.objects.find(o => o.zone === (scenario === 'cast' ? 'stack' : 'battlefield') && o.name === cardName);
      const postRequirements = state.crypto_requirements || state.cryptoRequirements || [];
      const openings = await services.current.buildLocalRequirementOpeningsForRequirements([...requirements, ...postRequirements], { forceZiffleOpeningProof: true });
      await services.current.verifyAuditOpeningsAgainstManifests(openings);
      const differences = (left, right, path = '') => {
        if (JSON.stringify(left) === JSON.stringify(right)) return [];
        if (left && right && typeof left === 'object' && typeof right === 'object') {
          return [...new Set([...Object.keys(left), ...Object.keys(right)])].flatMap(key => differences(left[key], right[key], `${path}.${key}`));
        }
        return [{ path, left, right }];
      };
      const withoutWorkerTiming = ({ __perf, ...checkpoint }) => checkpoint;
      let peerAuditDiff = [], paidAuditDiff = [];
      let peerAuditMatches = null;
      let paidAuditMatches = null;
      let paidDecision = null;
      if (peer) {
        const peerState = await peer.call('dispatch', command);
        const ownAudit = await call('exportPublicAuditCheckpoint');
        const peerAudit = await peer.call('exportPublicAuditCheckpoint');
        peerAuditDiff = differences(withoutWorkerTiming(ownAudit), withoutWorkerTiming(peerAudit));
        peerAuditMatches = await publicCheckpointHash(ownAudit) === await publicCheckpointHash(peerAudit);
        const confirmPayment = decision => ({ type: 'mana_payment', response: {
          action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash,
        } });
        const paid = await call('dispatch', confirmPayment(state.decision));
        refs.stateRef.current = paid;
        await peer.call('dispatch', confirmPayment(peerState.decision));
        paidDecision = paid.decision.kind;
        const ownPaidAudit = await call('exportPublicAuditCheckpoint');
        const peerPaidAudit = await peer.call('exportPublicAuditCheckpoint');
        paidAuditDiff = differences(withoutWorkerTiming(ownPaidAudit), withoutWorkerTiming(peerPaidAudit));
        paidAuditMatches = await publicCheckpointHash(ownPaidAudit) === await publicCheckpointHash(peerPaidAudit);
      }
      const opening = openings[0];
      const duplicate = manifests[1].slotSecrets.find(entry => entry.slot === 2);
      const rejected = async value => {
        try { await services.current.verifyAuditOpeningsAgainstManifests([value]); return ''; }
        catch (error) { return String(error.message); }
      };
      const duplicateError = await rejected({ ...opening, slot: 2, card: duplicate.card,
        salt: duplicate.salt, commitment: duplicate.commitment });
      const wrongOrigin = genesis.reveals.find(reveal => reveal.originalSlot === 2).cardPosition;
      const originError = await rejected({ ...opening, originPosition: wrongOrigin,
        originPositionCommitment: `ziffle:${genesis.deckHash}:${wrongOrigin}` });
      const missingOrigin = { ...opening };
      delete missingOrigin.originPosition; delete missingOrigin.originPositionCommitment;
      const missingOriginError = await rejected(missingOrigin);
      return { position, originPosition, originCommitment: `ziffle:${genesis.deckHash}:${originPosition}`, revealedInput: 2, originalSlot: 4, beforeSource: beforeOrder[2], afterObject: afterOrder[position],
        peerAuditDiff, paidAuditDiff, peerAuditMatches, paidAuditMatches, paidDecision, decision: state.decision.kind, currentMetadata: fieldRing?.hiddenCard,
        handId: handRing.id, fieldId: fieldRing?.id, retired: !afterCheckpoint.objects.some(o => o.id === handRing.id),
        preOpenings, openings, duplicateError, originError, missingOriginError };
    } finally { reactRoot?.unmount(); worker.terminate(); peer?.worker.terminate(); }
  }, { scenario, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
  assert.deepEqual(pageErrors, []);
  assert.notEqual(result.beforeSource, result.afterObject, 'cipher and engine permutations are independent');
  assert.notEqual(result.handId, result.fieldId, 'the real engine retires the hand object');
  assert.equal(result.retired, true);
  assert.equal(result.currentMetadata.originSlot, result.originPosition);
  assert.equal(result.currentMetadata.originCommitment, result.originCommitment);
  if (scenario === 'cast') {
    assert.equal(result.decision, 'mana_payment');
    assert.deepEqual(result.peerAuditDiff, [], 'both engine peers produce the same public checkpoint during mana payment');
    assert.equal(result.peerAuditMatches, true, 'pending-cast protocol hashes agree');
    assert.equal(result.paidDecision, 'priority');
    assert.equal(result.paidAuditMatches, true, 'completed-cast protocol hashes agree');
    assert.deepEqual(result.paidAuditDiff, [], 'both engine peers agree after the cast is paid and placed on the stack');
  }
  assert.match(result.duplicateError, /slot|commitment/i, 'another valid copy cannot replace the committed copy');
  assert.match(result.originError, /origin/i, 'a self-declared genesis anchor cannot replace engine metadata');
  assert.match(result.missingOriginError, /origin/i, 'anchored cards require the origin fields');
  assert.equal(result.preOpenings.length, 1);
  assert.ok(result.openings.length > 0);
  for (const opening of result.openings) {
    assert.equal(opening.slot, 4);
    assert.equal(opening.card, scenario === 'cast' ? 'Goblin Guide' : 'Barbarian Ring');
    assert.equal(opening.position, result.position);
    assert.equal(opening.originPosition, result.originPosition);
    assert.equal(opening.originPositionCommitment, result.originCommitment);
    assert.equal(opening.ziffleReveal.shuffleOriginalSlot ?? opening.ziffleReveal.originalSlot, 4);
  }
});
