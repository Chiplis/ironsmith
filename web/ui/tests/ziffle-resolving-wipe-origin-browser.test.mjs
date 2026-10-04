import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('committed wipe retains its trusted origin after graveyard ordering resumes resolution', { timeout: 120000 }, async t => {
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
    const cardName = 'Wrath of the Skies';
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
      for (let index = 0; index < 4; index++) await call('addCardToZone', 1, 'Plains', 'battlefield', true);
      for (let index = 0; index < 2; index++) await call('addCardToZone', 0, 'Grizzly Bears', 'battlefield', true);
      state = await call('uiState');
      const handWrath = state.players[1].hand_cards.find(card => card.name === cardName);
      if (!handWrath) throw new Error('Draw did not produce committed Wrath');
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
      const action = state.decision.actions.find(a => a.action_ref?.kind === 'cast_spell' && Number(a.object_id) === handWrath.id);
      if (!action) throw new Error('Wrath is not playable');
      const castCommand = { type: 'priority_action', action_ref: action.action_ref, object_id: handWrath.id };
      const castRequirements = await call('previewCryptoRequirements', castCommand);
      const castOpenings = await services.current.buildLocalOpeningsForCommand(castCommand, castRequirements);
      peer = createWorkerSession(); await peer.ready;
      await peer.call('setPerspective', 0);
      for (const [method, args] of setupCommands) await peer.call(method, ...args);
      recordingSetup = false;
      const decisions = [];
      const dispatchBoth = async command => {
        state = await call('dispatch', command);
        refs.stateRef.current = state;
        const peerState = await peer.call('dispatch', command);
        decisions.push(state.decision);
        if (state.decision?.kind !== peerState.decision?.kind) throw new Error('Peer decision differs');
        return state;
      };
      await dispatchBoth(castCommand);
      if (state.decision?.kind !== 'number') throw new Error(`Expected X selection: ${JSON.stringify(state.decision)}`);
      await dispatchBoth({ type: 'number_choice', value: 2 });
      if (state.decision?.kind !== 'mana_payment') throw new Error(`Expected payment: ${JSON.stringify(state.decision)}`);
      await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash } });
      for (let index = 0; index < 8 && state.decision?.reason !== 'Ordering'; index++) {
        const decision = state.decision;
        if (decision?.kind === 'priority') {
          const pass = decision.actions.find(a => a.action_ref?.kind === 'pass_priority');
          if (!pass) throw new Error('Missing pass action');
          await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
        } else if (decision?.kind === 'select_options' && decision.player === 1) {
          await dispatchBoth({ type: 'select_options', option_indices: [decision.options.find(option => option.description === 'Yes').index] });
        } else if (decision?.kind === 'number' && decision.player === 1) {
          await dispatchBoth({ type: 'number_choice', value: 2 });
        } else throw new Error(`Unexpected resolution decision: ${JSON.stringify({decision,decisions})}`);
      }
      if (state.decision?.reason !== 'Ordering' || state.decision?.player !== 0) throw new Error(`Expected opponent ordering: ${JSON.stringify(state.decision)}`);
      const ordering = state.decision;
      const command = { type: 'select_options', option_indices: ordering.options.map(option => option.index) };
      const orderingCheckpoint = await call('getHiddenCardState');
      const requirements = await call('previewCryptoRequirements', command);
      const beforeOpening = castOpenings[0];
      const trustedBefore = await services.current.currentZiffleOriginForOpening(beforeOpening);
      const captureError = async work => {
        try { return { value: await work(), error: null }; }
        catch (error) { return { value: null, error: String(error.message) }; }
      };
      const preVerification = await captureError(() => services.current.verifyAuditOpeningsAgainstManifests([beforeOpening]));
      // Resolving spells are popped from the visible stack while nested choices
      // are pending. The source must still be exported as trusted identity here.
      const preOpeningBuild = await captureError(() => services.current.buildLocalRequirementOpeningsForRequirements(requirements, { forceZiffleOpeningProof: true }));
      const preparedOpenings = preOpeningBuild.value || [];
      const preparedPreVerification = await captureError(() => services.current.verifyAuditOpeningsAgainstManifests(preparedOpenings));
      const wrongOrigin = genesis.reveals.find(reveal => reveal.originalSlot === 2).cardPosition;
      const withWrongOrigin = opening => ({ ...opening,
        originPosition: wrongOrigin,
        originPositionCommitment: `ziffle:${genesis.deckHash}:${wrongOrigin}`,
      });
      const wrongOriginBefore = await captureError(() => services.current.verifyAuditOpeningsAgainstManifests([withWrongOrigin(beforeOpening)]));
      await dispatchBoth(command);
      const afterCheckpoint = await call('getHiddenCardState');
      const trustedAfter = await services.current.currentZiffleOriginForOpening(beforeOpening);
      // Verify the opening prepared before the ordering command against the
      // receiving engine after it has resumed resolution and moved the spell.
      const ownerGame = refs.gameRef.current;
      const ownerSeat = refs.multiplayerRef.current.localPlayerIndex;
      const ownerState = refs.stateRef.current;
      let postVerification, postApplication, wrongOriginAfter, peerCheckpoint;
      try {
        refs.gameRef.current = new Proxy({}, { get: (_, method) => method.startsWith('ziffle')
          ? async input => api[method](input)
          : (...args) => peer.call(method, ...args) });
        refs.multiplayerRef.current.localPlayerIndex = 0;
        refs.stateRef.current = await peer.call('uiState');
        // Recheck the prepared opening after the source's runtime ID changes.
        // Public moves now prepare pre-timed openings; use its actual phase so
        // the forged-origin check exercises validation instead of filtering it.
        const revealOptions = { timing: preparedOpenings[0]?.timing || 'pre', updateState: false, previewInspector: false };
        wrongOriginAfter = await captureError(() => services.current.revealAuditOpenings([withWrongOrigin(preparedOpenings[0])], revealOptions));
        postVerification = await captureError(() => services.current.verifyAuditOpeningsAgainstManifests(preparedOpenings));
        postApplication = await captureError(() => services.current.revealAuditOpenings(preparedOpenings, revealOptions));
        peerCheckpoint = await peer.call('getHiddenCardState');
      } finally {
        refs.gameRef.current = ownerGame;
        refs.multiplayerRef.current.localPlayerIndex = ownerSeat;
        refs.stateRef.current = ownerState;
      }
      const postOpeningBuild = await captureError(() => services.current.buildLocalRequirementOpeningsForRequirements([
        ...requirements, ...(state.crypto_requirements || state.cryptoRequirements || []),
      ], { forceZiffleOpeningProof: true }));
      const ownAudit = await call('exportPublicAuditCheckpoint');
      const peerAudit = await peer.call('exportPublicAuditCheckpoint');
      return {
        decisions, ordering, beforeOpening, requirements, trustedBefore, trustedAfter,
        beforeWrath: orderingCheckpoint.objects.filter(object => object.name === cardName),
        afterWrath: afterCheckpoint.objects.filter(object => object.name === cardName),
        afterBears: afterCheckpoint.objects.filter(object => object.name === 'Grizzly Bears'),
        peerWrath: peerCheckpoint.objects.filter(object => object.name === cardName),
        peerBears: peerCheckpoint.objects.filter(object => object.name === 'Grizzly Bears'),
        postApplication, wrongOriginBefore, wrongOriginAfter,
        preVerification, preOpeningBuild, preparedPreVerification, postVerification, postOpeningBuild, finalDecision: state.decision?.kind,
        peerAuditMatches: await publicCheckpointHash(ownAudit) === await publicCheckpointHash(peerAudit),
      };
    } finally { reactRoot?.unmount(); worker.terminate(); peer?.worker.terminate(); }
  }, { verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
  assert.deepEqual(pageErrors, []);

  assert.equal(result.finalDecision, 'priority');
  assert.equal(result.peerAuditMatches, true, 'the receiving peer matches after applying the opening');
  assert.equal(result.preOpeningBuild.error, null);
  assert.equal(result.preparedPreVerification.error, null);
  assert.equal(result.preOpeningBuild.value.length, 1);
  assert.equal(result.postVerification.error, null, 'the opening prepared during graveyard ordering stays bound to its trusted origin after resolution');
  assert.equal(result.postApplication.error, null, 'the receiver applies the prepared public opening after resolution');
  assert.match(result.wrongOriginBefore.error, /origin/i, 'a different genesis anchor is rejected while resolution is pending');
  assert.match(result.wrongOriginAfter.error, /origin/i, 'a different genesis anchor is rejected after resolution');
  const prepared = result.preOpeningBuild.value[0];
  assert.equal(prepared.originPosition, result.beforeOpening.originPosition);
  assert.equal(prepared.originPositionCommitment, result.beforeOpening.originPositionCommitment);
  assert.equal(result.preVerification.error, null, 'existing anchored openings remain verifiable during nested resolution');
  assert.equal(result.beforeWrath.length, 1, 'the pending resolving spell remains available as trusted native metadata');
  assert.equal(result.trustedBefore.originPosition, result.trustedAfter.originPosition);
  assert.equal(result.postOpeningBuild.error, null, 'post-resolution openings preserve trusted origin binding');
  for (const cards of [result.afterWrath, result.peerWrath]) {
    assert.equal(cards.length, 1);
    assert.equal(cards[0].zone, 'graveyard');
  }
  for (const bears of [result.afterBears, result.peerBears]) {
    assert.equal(bears.length, 2);
    assert.ok(bears.every(object => object.zone === 'graveyard'), 'both destroyed creatures moved to the graveyard');
  }
});
