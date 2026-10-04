import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

for (const scenario of [
  { cardName: 'Phantasmal Image', copyName: 'Grizzly Bears' },
]) test(`committed ${scenario.cardName} retains its committed identity after copying ${scenario.copyName}`,  { timeout: 120000 }, async t => {
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
    const { cardName, copyName } = scenario;
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
      await call('addCardToZone', 1, 'Island', 'battlefield', true);
      await call('addCardToZone', 1, 'Island', 'battlefield', true);
      await call('addCardToZone', 1, 'Grizzly Bears', 'battlefield', true);
      state = await call('uiState');
      const handCard = state.players[1].hand_cards.find(card => card.name === cardName);
      if (!handCard) throw new Error('Draw did not produce committed copy spell');
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
      const action = state.decision.actions.find(a => a.action_ref?.kind === 'cast_spell' && Number(a.object_id) === handCard.id);
      if (!action) throw new Error(`Card not playable: ${JSON.stringify({ handCard, decision: state.decision })}`);
      const command = { type: 'priority_action', action_ref: action.action_ref, object_id: handCard.id };
      const requirements = await call('previewCryptoRequirements', command);
      const preOpenings = await services.current.buildLocalOpeningsForCommand(command, requirements);
      {
        peer = createWorkerSession(); await peer.ready;
        // Independently reproduce setup and the announced public card.
        await peer.call('setPerspective', 0);
        for (const [method, args] of setupCommands) await peer.call(method, ...args);
      }
      recordingSetup = false;
      state = await call('dispatch', command); refs.stateRef.current = state;
      await peer.call('dispatch', command);
      const firstDecision = state.decision;
      const decisions = [];
      for (let index = 0; index < 15; index++) {
        const checkpoint = await call('getHiddenCardState');
        if (checkpoint.objects.some(o => o.zone === 'battlefield' && o.name === copyName && o.hiddenCard?.slot === 4)) break;
        const decision = state.decision;
        decisions.push(decision);
        let next;
        if (decision.kind === 'mana_payment') next = { type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } };
        else if (decision.kind === 'priority') next = { type: 'priority_action', action_ref: decision.actions.find(a => a.action_ref?.kind === 'pass_priority').action_ref };
        else if (decision.kind === 'select_options') next = { type: 'select_options', option_indices: [decision.options.find(o => /yes|Grizzly Bears/i.test(o.description))?.index ?? decision.options[0].index] };
        else if (decision.kind === 'select_objects') {
          const candidate = decision.objects?.find(o => o.name === copyName) || decision.candidates?.find(o => o.name === copyName);
          if (!candidate) throw new Error(`No copy target: ${JSON.stringify(decision)}`);
          next = { type: 'select_objects', object_ids: [candidate.id] };
        } else throw new Error(`Unexpected copy decision: ${JSON.stringify(decision)}`);
        requirements.push(...await call('previewCryptoRequirements', next));
        state = await call('dispatch', next); refs.stateRef.current = state;
        await peer.call('dispatch', next);
      }
      const afterCheckpoint = await call('getHiddenCardState');
      const fieldCard = afterCheckpoint.objects.find(o => o.zone === 'battlefield' && o.name === copyName && o.hiddenCard?.slot === 4);
      if (!fieldCard) throw new Error(`Copy did not enter battlefield: ${JSON.stringify({decisions, objects:afterCheckpoint.objects})}`);
      const postRequirements = state.crypto_requirements || state.cryptoRequirements || [];
      const openings = await services.current.buildLocalRequirementOpeningsForRequirements([...requirements, ...postRequirements], { forceZiffleOpeningProof: true, timing: 'post' });
      await services.current.verifyAuditOpeningsAgainstManifests(openings);
      const peerState = await peer.call('uiState');
      const beforeApply = await peer.call('getHiddenCardState');
      const ownerHashBefore = await publicCheckpointHash(await call('exportPublicAuditCheckpoint'));
      const peerHashBefore = await publicCheckpointHash(await peer.call('exportPublicAuditCheckpoint'));
      refs.gameRef.current = new Proxy({}, { get: (_, method) => method.startsWith('ziffle')
        ? async input => api[method](input) : (...args) => peer.call(method, ...args) });
      refs.multiplayerRef.current.localPlayerIndex = 0;
      refs.stateRef.current = peerState;
      const revealOptions = { timing: 'post', updateState: false, previewInspector: false };
      const originalOpening = openings[0];
      const duplicate = manifests[1].slotSecrets.find(entry => entry.slot === 2);
      const wrongOriginPosition = genesis.reveals.find(reveal => reveal.originalSlot === 2).cardPosition;
      const rejectedOpeningError = async opening => {
        try { await services.current.revealAuditOpenings([opening], revealOptions); return null; }
        catch (error) { return String(error.message); }
      };
      const forgedIdentityError = await rejectedOpeningError({ ...originalOpening, card: copyName });
      const swappedCopyError = await rejectedOpeningError({ ...originalOpening,
        slot: duplicate.slot, card: duplicate.card, salt: duplicate.salt, commitment: duplicate.commitment });
      const wrongOriginError = await rejectedOpeningError({ ...originalOpening,
        originPosition: wrongOriginPosition,
        originPositionCommitment: `ziffle:${genesis.deckHash}:${wrongOriginPosition}` });
      const afterRejected = await peer.call('getHiddenCardState');
      const rejectedOpeningsPreserveObjects = JSON.stringify(beforeApply.objects) === JSON.stringify(afterRejected.objects);
      let applyError = null;
      try { await services.current.revealAuditOpenings(openings, revealOptions); }
      catch (error) { applyError = error.message; }
      const afterApply = await peer.call('getHiddenCardState');
      const ownAudit = await call('exportPublicAuditCheckpoint');
      const peerAudit = await peer.call('exportPublicAuditCheckpoint');
      const peerAuditMatches = await publicCheckpointHash(ownAudit) === await publicCheckpointHash(peerAudit);
      const peerStateAfter = await peer.call('uiState');
      const compactObject = (object, ui) => {
        if (!object) return null;
        const visible = ui.players.flatMap(player => player.battlefield).find(card => card.id === object.id);
        return { id: object.id, owner: object.hiddenCard?.owner, controller: visible?.controller,
          zone: object.zone, name: object.name, tapped: visible?.tapped, hiddenCard: object.hiddenCard };
      };
      const peerBefore = beforeApply.objects.find(o => o.id === fieldCard.id);
      const peerAfter = afterApply.objects.find(o => o.id === fieldCard.id);
      return { cardName, copyName, decisions, firstDecision: firstDecision.kind, finalDecision: state.decision.kind,
        handCard: { id: handCard.id, name: handCard.name },
        preOpenings: preOpenings.map(({ owner, slot, card, objectId, timing, position, originPosition }) =>
          ({ owner, slot, card, objectId, timing, position, originPosition })),
        openings: openings.map(({ owner, slot, card, objectId, timing, position, originPosition }) =>
          ({ owner, slot, card, objectId, timing, position, originPosition })),
        proofsVerified: true, applyError, forgedIdentityError, swappedCopyError, wrongOriginError, rejectedOpeningsPreserveObjects, peerAuditMatches, beforeHydrationHashesMatch: ownerHashBefore === peerHashBefore,
        allPeerObjectsPreserved: JSON.stringify(beforeApply.objects) === JSON.stringify(afterApply.objects),
        ownerCard: compactObject(fieldCard, state), peerBefore: compactObject(peerBefore, peerState), peerAfter: compactObject(peerAfter, peerStateAfter),
      };
    } finally { reactRoot?.unmount(); worker.terminate(); peer?.worker.terminate(); }
  }, { scenario, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
  assert.deepEqual(pageErrors, []);
  assert.equal(result.handCard.name, scenario.cardName);
  assert.equal(result.finalDecision, 'priority');
  assert.equal(result.proofsVerified, true, 'the actual Ziffle opening proof verifies');
  assert.equal(result.beforeHydrationHashesMatch, true, 'both engines agree before remote hydration');
  assert.equal(result.peerAuditMatches, true, 'both peers agree after successful public opening application');
  assert.equal(result.ownerCard.name, scenario.copyName);
  assert.equal(result.ownerCard.zone, 'battlefield');
  assert.equal(result.ownerCard.tapped, false);
  assert.notEqual(result.handCard.id, result.ownerCard.id);
  assert.equal(result.openings.length, 1);
  assert.equal(result.openings[0].card, scenario.cardName);
  assert.equal(result.openings[0].timing, 'post');
  assert.equal(result.openings[0].originPosition, result.ownerCard.hiddenCard.originSlot);
  assert.deepEqual(result.peerBefore, result.ownerCard);
  assert.deepEqual(result.peerAfter, result.peerBefore);
  assert.match(result.forgedIdentityError, /deck commitment/i, 'a current display name cannot replace committed printed identity');
  assert.match(result.swappedCopyError, /slot|commitment/i, 'another valid same-name copy cannot replace this committed copy');
  assert.match(result.wrongOriginError, /origin/i, 'another genesis position cannot replace the trusted origin');
  assert.equal(result.rejectedOpeningsPreserveObjects, true, 'rejected forged openings do not mutate receiver objects');
  assert.equal(result.applyError, null, 'the receiver accepts the committed identity without overwriting current characteristics');
  assert.equal(result.allPeerObjectsPreserved, true, 'reopening a public identity preserves every current object');

});
