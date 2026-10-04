import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('committed Adventure opening preserves Stomp while choosing its targets', { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  const pageErrors = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  await page.route('**/opening-zone-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root"></div>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/opening-zone-test`);
  const result = await page.evaluate(async ({ verifierUrl }) => {
    const cardName = 'Bonecrusher Giant';
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
          if (data.type === 'error') return reject(new Error(data.error.stack || data.error.message));
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
    const setupMethods = new Set(['startMatch', 'dispatch', 'revealHiddenPosition', 'drawCard', 'addCardToZone']);
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
          const step = api.ziffleBuildShuffleStep({ deckCount, context, keys, steps, shuffler, entropyHex: ['33', '44'][shuffler].repeat(32) });
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
      if (!beforeOrder.includes(originalFourId)) throw new Error('Original slot4 not in library');
      const ceremony = genesis;
      const { deckCount } = genesis;
      const position = originPosition;
      const afterOrder = beforeOrder;
      const secret = manifests[1].slotSecrets.find(entry => entry.slot === 4);
      const positionCommitment = `ziffle:${ceremony.deckHash}:${position}`;
      await call('revealHiddenPosition', { owner: 1, objectId: afterOrder[position], position,
        originalSlot: 4, cardName: secret.card, positionCommitment, commitment: secret.commitment });
      for (let index = deckCount - 1; index >= position; index--) await call('drawCard', 1);
      for (let index = 0; index < 4; index++) await call('addCardToZone', 1, 'Mountain', 'battlefield', true);
      for (let index = 0; index < 2; index++) await call('addCardToZone', 0, 'Grizzly Bears', 'battlefield', true);
      state = await call('uiState');
      const handAdventure = state.players[1].hand_cards.find(card => card.name === cardName);
      if (!handAdventure) throw new Error('Draw did not produce committed Adventure');
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
      const services = { current: { collectZiffleRevealTokens: async (_selected, p) => genesis.tokens.filter(token => token.cardPosition === p),
        collectZiffleRevealTokensBatch: async (_selected, ps) => genesis.tokens.filter(token => ps.includes(token.cardPosition)),
        commandObjectHiddenRefs: () => [], commandObjectStableIds: () => [],
        currentObjectIdForHiddenRef: async () => null, currentObjectIdForStableId: async () => null } };
      reactRoot = mountOpeningServices(base, services);
      const casts = state.decision.actions.filter(a => a.action_ref?.kind === 'cast_spell');
      const action = casts.find(a => a.label.includes('Stomp'));
      if (!action) throw new Error(`No Adventure cast: ${JSON.stringify(casts)}`);
      const command = { type: 'priority_action', action_ref: action.action_ref, object_id: handAdventure.id };
      const requirements = await call('previewCryptoRequirements', command);
      const openings = await services.current.buildLocalOpeningsForCommand(command, requirements);
      peer = createWorkerSession(); await peer.ready;
      await peer.call('setPerspective', 0);
      for (const [method, args] of setupCommands) await peer.call(method, ...args);
      recordingSetup = false;
      state = await call('dispatch', command); refs.stateRef.current = state;
      await peer.call('dispatch', command);
      const postRequirements = state.crypto_requirements || state.cryptoRequirements || [];
      const postOpenings = await services.current.buildLocalRequirementOpeningsForRequirements(postRequirements, { forceZiffleOpeningProof: true, timing: 'post' });
      await services.current.verifyAuditOpeningsAgainstManifests(postOpenings);
      const ownerCheckpoint = await call('getHiddenCardState');
      const ownAudit = await call('exportPublicAuditCheckpoint');
      const peerAudit = await peer.call('exportPublicAuditCheckpoint');
      refs.gameRef.current = new Proxy({}, { get: (_, method) => method.startsWith('ziffle')
        ? async input => api[method](input) : (...args) => peer.call(method, ...args) });
      refs.multiplayerRef.current.localPlayerIndex = 0;
      refs.stateRef.current = await peer.call('uiState');
      const beforeApply = await peer.call('getHiddenCardState');
      const revealOptions = { timing: 'post', updateState: false, previewInspector: false };
      const originalOpening = postOpenings[0];
      const duplicate = manifests[1].slotSecrets.find(entry => entry.slot === 2);
      const wrongOriginPosition = genesis.reveals.find(reveal => reveal.originalSlot === 2).cardPosition;
      const rejectedOpeningError = async opening => {
        try { await services.current.revealAuditOpenings([opening], revealOptions); return null; }
        catch (error) { return String(error.message); }
      };
      const forgedIdentityError = await rejectedOpeningError({ ...originalOpening, card: 'Stomp' });
      const swappedCopyError = await rejectedOpeningError({ ...originalOpening,
        slot: duplicate.slot, card: duplicate.card, salt: duplicate.salt, commitment: duplicate.commitment });
      const wrongOriginError = await rejectedOpeningError({ ...originalOpening,
        originPosition: wrongOriginPosition,
        originPositionCommitment: `ziffle:${genesis.deckHash}:${wrongOriginPosition}` });
      const afterRejected = await peer.call('getHiddenCardState');
      const rejectedOpeningsPreserveObjects = JSON.stringify(beforeApply.objects) === JSON.stringify(afterRejected.objects);
      let applyError = null;
      try { await services.current.revealAuditOpenings(postOpenings, revealOptions); }
      catch (error) { applyError = error.message; }
      const afterApply = await peer.call('getHiddenCardState');
      return { castLabel: action.label, decisionKind: state.decision.kind, decisionSource: state.decision.source_name,
        preOpeningCount: openings.length, postRequirements, postOpeningCount: postOpenings.length,
        openingNames: postOpenings.map(opening => opening.card), proofVerificationPassed: true,
        source: ownerCheckpoint.objects.find(object => object.name === 'Stomp'),
        peerSource: beforeApply.objects.find(object => object.name === 'Stomp'), applyError,
        peerSourceAfter: afterApply.objects.find(object => object.name === 'Stomp'),
        forgedIdentityError, swappedCopyError, wrongOriginError, rejectedOpeningsPreserveObjects,
        peersAgreeBeforeOpening: await publicCheckpointHash(ownAudit) === await publicCheckpointHash(peerAudit),
        peersAgreeAfterOpening: await publicCheckpointHash(await call('exportPublicAuditCheckpoint')) === await publicCheckpointHash(await peer.call('exportPublicAuditCheckpoint')),
        receiverObjectsUnchangedAfterOpening: JSON.stringify(beforeApply.objects) === JSON.stringify(afterApply.objects),
      };
    } finally { reactRoot?.unmount(); worker.terminate(); peer?.worker.terminate(); }
  }, { verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
  assert.deepEqual(pageErrors, []);
  assert.equal(result.decisionKind, 'targets');
  assert.equal(result.decisionSource, 'Stomp');
  assert.ok(result.preOpeningCount > 0);
  assert.ok(result.postOpeningCount > 0, 'the real command requires a post-action opening');
  assert.deepEqual(result.openingNames, ['Bonecrusher Giant']);
  assert.equal(result.proofVerificationPassed, true);
  assert.equal(result.peersAgreeBeforeOpening, true);
  assert.equal(result.source.zone, 'stack');
  assert.deepEqual(result.peerSource, result.source);
  assert.ok(result.source.hiddenCard.originCommitment.startsWith('ziffle:'));
  assert.match(result.forgedIdentityError, /deck commitment/i, 'Stomp cannot replace the committed Bonecrusher Giant identity');
  assert.match(result.swappedCopyError, /slot|commitment/i, 'another Bonecrusher Giant copy cannot replace the committed copy');
  assert.match(result.wrongOriginError, /origin/i, 'a forged genesis origin is rejected');
  assert.equal(result.rejectedOpeningsPreserveObjects, true);
  assert.equal(result.applyError, null, 'the receiver accepts the committed creature identity while the Adventure face is on the stack');
  assert.equal(result.receiverObjectsUnchangedAfterOpening, true);
  assert.equal(result.peersAgreeAfterOpening, true);
  assert.deepEqual(result.peerSourceAfter, result.source);

});
