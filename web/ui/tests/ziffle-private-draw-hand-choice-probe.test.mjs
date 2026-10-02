import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const isolatedWasmPackage = process.env.IRONSMITH_TEST_WASM_PKG;

const scenarios = [
  ...['Brainstorm', 'Faithless Looting', 'Spelunking', 'Collected Company'].map(cardName => ({ cardName })),
  { cardName: 'Wolf-Skull Shaman', topName: 'Llanowar Elves', acceptKinship: true },
  { cardName: 'Wolf-Skull Shaman', topName: 'Llanowar Elves', acceptKinship: false },
  { cardName: 'Wolf-Skull Shaman', topName: 'Grizzly Bears', acceptKinship: false },
];
for (const { cardName, topName, acceptKinship } of scenarios) test(`${cardName} keeps committed identities across private draws and hand choices${topName ? ` (${topName}, ${acceptKinship ? 'reveal' : 'decline'})` : ''}`, { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    ...(isolatedWasmPackage ? { resolve: { alias: [{
      find: /^.*wasm_demo\/pkg\/(.*)$/,
      replacement: `${path.resolve(isolatedWasmPackage)}/$1`,
    }] } } : {}),
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null,
      ...(isolatedWasmPackage ? { fs: { allow: [path.resolve(root, '../..'), path.resolve(isolatedWasmPackage)] } } : {}) } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  const pageErrors = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  await page.route('**/opening-zone-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root"></div>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/opening-zone-test`);
  const result = await page.evaluate(async ({ verifierUrl, cardName, topName, acceptKinship }) => {
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
          if (data.type === 'priorityAnalysis') { analyses.set(data.revision, data.decision); analysisWaiters.get(data.revision)?.(data.decision); return; }
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
    const { worker, call, ready } = createWorkerSession();
    let peer;
    let reactRoot;
    try {
      await ready;
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
      const originPosition = genesis.deckCount - 1;
      const spellSlot = genesis.reveals.find(reveal => reveal.cardPosition === originPosition).originalSlot;
      const deck = Array(genesis.deckCount).fill('Island');
      deck[spellSlot] = cardName;
      if (topName) {
        const topSlot = genesis.reveals.find(reveal => reveal.cardPosition === originPosition - 1).originalSlot;
        deck[topSlot] = topName;
      }
      const manifests = await Promise.all([0, 1].map(owner => buildPrivateDeckManifest({ matchId: 'zone-opening', owner, deck })));

      await call('setPerspective', 1);
      let state = await call('startMatch', { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1, format: 'normal',
        startingPlayer: 1, openingHandSize: 0, decks: [[], []], publicDecklists: [deck, deck], hiddenDeckManifests: manifests.map((manifest, owner) => buildZiffleRuntimeManifest(manifest, { ...genesis, owner })) });
      for (let i = 0; i < 30 && state.phase !== 'first main phase'; i++) {
        const action = state.decision?.actions?.find(a => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(a.action_ref?.kind));
        if (!action) throw new Error(`Unexpected pregame ${JSON.stringify(state.decision)}`);
        state = await call('dispatch', { type: 'priority_action', action_ref: action.action_ref });
      }
      if (state.phase !== 'first main phase') throw new Error('Main phase missing');
      const beforeCheckpoint = await call('exportSyncCheckpoint');
      const beforeOrder = [...beforeCheckpoint.players[1].library];
      const originalFourId = beforeCheckpoint.objects.find(o => o.owner === 1 && o.hiddenCard?.slot === originPosition)?.id;
      if (!beforeOrder.includes(originalFourId)) throw new Error('Committed spell not in library');
      const ceremony = genesis;
      const { deckCount } = genesis;
      const position = originPosition;
      const afterOrder = beforeOrder;
      const secret = manifests[1].slotSecrets.find(entry => entry.slot === spellSlot);
      const positionCommitment = `ziffle:${ceremony.deckHash}:${position}`;
      await call('revealHiddenPosition', { owner: 1, objectId: afterOrder[position], position,
        originalSlot: spellSlot, cardName: secret.card, positionCommitment, commitment: secret.commitment });
      for (let index = deckCount - 1; index >= position; index--) await call('drawCard', 1);
      const manaLand = cardName === 'Brainstorm' ? 'Island' : cardName === 'Faithless Looting' ? 'Mountain' : 'Forest';
      const manaCount = cardName === 'Collected Company' ? 4 : cardName === 'Spelunking' ? 3 : topName ? 2 : 1;
      for (let index = 0; index < manaCount; index++) await call('addCardToZone', 1, manaLand, 'battlefield', true);
      state = await call('uiState');
      const handSpell = state.players[1].hand_cards.find(card => card.name === cardName);
      if (!handSpell) throw new Error('Draw did not produce the committed spell');
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
        commandObjectHiddenRefs: command => command.object_hidden_refs || [], commandObjectStableIds: command => command.object_stable_ids || [],
        currentObjectIdForHiddenRef: async () => null, currentObjectIdForStableId: async () => null } };
      reactRoot = mountOpeningServices(base, services);
      peer = createWorkerSession(); await peer.ready;
      await peer.call('importSyncCheckpoint', await call('exportSyncCheckpoint'));
      await peer.call('setPerspective', 0);
      const ownerGame = refs.gameRef.current;
      const peerGame = new Proxy({}, { get: (_, method) => method.startsWith('ziffle')
        ? async input => api[method](input)
        : (...args) => peer.call(method, ...args) });
      const stages = [];
      let stage = 'setup';
      let pendingRequirements = [];
      const publicHash = async engineCall => publicCheckpointHash(await engineCall('exportPublicAuditCheckpoint'));
      const onPeer = async work => {
        const savedState = refs.stateRef.current;
        try {
          refs.gameRef.current = peerGame;
          refs.multiplayerRef.current.localPlayerIndex = 0;
          refs.stateRef.current = await peer.call('uiState');
          return await work();
        } finally {
          refs.gameRef.current = ownerGame;
          refs.multiplayerRef.current.localPlayerIndex = 1;
          refs.stateRef.current = savedState;
        }
      };
      // Use verified genesis tokens to hydrate only the owner's worker. This
      // probes engine/proof/application boundaries without the network transport.
      const hydratePrivate = async requirements => {
        const privateRequirements = requirements.filter(value => value.type === 'private_open' && value.owner === 1);
        const reveals = [];
        const checkpoint = await call('exportSyncCheckpoint');
        for (const requirement of privateRequirements) {
          const currentObject = checkpoint.objects.find(object => object.id === requirement.objectId)
            || checkpoint.objects.find(object => object.owner === requirement.owner
              && object.hiddenCard?.commitment === requirement.commitment);
          if (currentObject && currentObject.name !== 'Hidden Card') continue;
          const position = requirement.publicSlot ?? requirement.slot;
          const reveal = api.ziffleRevealCard({ deckCount, context: genesis.context, keys: genesis.keys, steps: genesis.steps,
            cardPosition: position, tokens: genesis.tokens.filter(token => token.cardPosition === position) });
          const secret = manifests[1].slotSecrets.find(entry => entry.slot === reveal.originalSlot);
          reveals.push({ owner: 1, position, originalSlot: reveal.originalSlot, cardName: secret.card,
            positionCommitment: requirement.publicCommitment ?? requirement.commitment, commitment: secret.commitment });
        }
        if (reveals.length) {
          await call('revealHiddenPositions', { reveals, recomputeDecision: true });
          state = await call('uiState'); refs.stateRef.current = state;
        }
        return reveals.length;
      };
      const applyPublic = async (openings, timing) => onPeer(() => services.current.revealAuditOpenings(openings,
        { timing, updateState: false, previewInspector: false }));
      const dispatchBoth = async (command, label) => {
        stage = `${label}:preview`;
        const requirements = await call('previewCryptoRequirements', command);
        pendingRequirements = requirements;
        stage = `${label}:private-hydration`;
        const privateCount = await hydratePrivate(requirements);
        stage = `${label}:opening-build`;
        const commandOpenings = await services.current.buildLocalOpeningsForCommand(command, requirements);
        const requirementOpenings = await services.current.buildLocalRequirementOpeningsForRequirements(requirements,
          { forceZiffleOpeningProof: true });
        const openings = [...commandOpenings, ...requirementOpenings];
        stage = `${label}:pre-opening-apply`;
        await applyPublic(openings, 'pre');
        stage = `${label}:owner-dispatch`;
        state = await call('dispatch', command); refs.stateRef.current = state;
        stage = `${label}:peer-dispatch`;
        const peerState = await peer.call('dispatch', command);
        stage = `${label}:post-opening-apply`;
        await applyPublic(openings, 'post');
        stage = `${label}:post-private-hydration`;
        const postPrivateCount = await hydratePrivate(state.crypto_requirements || []);
        stages.push({ label, command, requirements, privateCount, postPrivateCount,
          decision: state.decision, peerDecision: peerState.decision,
          openings: openings.map(({ ziffleReveal, salt, ...opening }) => ({ ...opening, proof: Boolean(ziffleReveal), salted: Boolean(salt) })),
          hashesEqual: await publicHash(call) === await publicHash(peer.call) });
        return state;
      };
      const playerSummary = checkpoint => ({ hand: checkpoint.players[1].hand.length, library: checkpoint.players[1].library.length,
        graveyard: checkpoint.objects.filter(object => object.owner === 1 && object.zone === 'graveyard').map(object => object.name),
        exile: checkpoint.objects.filter(object => object.owner === 1 && object.zone === 'exile').map(object => object.name),
        privateCards: checkpoint.objects.filter(object => object.owner === 1 && object.zone === 'hand').map(object => object.name) });
      const resolveCast = async (action, prefix = '') => {
        await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: action.object_id }, `${prefix}cast`);
        if (state.decision?.kind !== 'mana_payment') throw new Error(`Expected payment: ${JSON.stringify(state.decision)}`);
        await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash } }, `${prefix}payment`);
        for (let index = 0; index < (cardName === 'Spelunking' ? 4 : 2); index++) {
          const pass = state.decision?.actions?.find(a => a.action_ref?.kind === 'pass_priority');
          if (!pass) throw new Error(`Missing resolution pass: ${JSON.stringify(state.decision)}`);
          await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref }, `${prefix}resolve-pass-${index}`);
        }
        for (let index = 0; index < 8 && state.decision?.kind !== 'priority'; index++) {
          const decision = state.decision;
          if (decision.kind === 'select_objects') {
            const chosen = decision.candidates.filter(candidate => candidate.legal).slice(0, cardName === 'Spelunking' ? 1 : decision.min);
            const command = { type: 'select_objects', object_ids: chosen.map(candidate => candidate.id),
              object_hidden_refs: chosen.map(candidate => ({ owner: candidate.hidden_ref.owner, zone: candidate.hidden_ref.zone,
                public_slot: candidate.hidden_ref.public_slot, public_commitment: candidate.hidden_ref.public_commitment })) };
            await dispatchBoth(command, `${prefix}hand-choice-${index}`);
          } else if (decision.kind === 'select_options') {
            await dispatchBoth({ type: 'select_options', option_indices: decision.options.filter(option => option.legal).slice(0, decision.min).map(option => option.index) }, `${prefix}ordering-${index}`);
          } else throw new Error(`Unexpected resolution choice: ${JSON.stringify(decision)}`);
        }
      };
      try {
        const action = state.decision.actions.find(a => a.action_ref?.kind === 'cast_spell' && Number(a.object_id) === handSpell.id);
        if (!action) throw new Error('Committed spell is not playable');
        await resolveCast(action);
        const normalResolution = { owner: playerSummary(await call('exportSyncCheckpoint')),
          peer: playerSummary(await peer.call('exportSyncCheckpoint')), hashesEqual: await publicHash(call) === await publicHash(peer.call) };
        if (topName) {
          // Resume both complete engine views at the end of Alice's turn, so
          // Bob's next upkeep generates the Kinship trigger normally.
          for (const engineCall of [call, peer.call]) {
            const checkpoint = await engineCall('exportSyncCheckpoint');
            checkpoint.turn = { ...checkpoint.turn, activePlayer: 0, priorityPlayer: 0,
              turnNumber: 2, phase: 'ending', step: 'end' };
            checkpoint.priorityRuntime = { ...checkpoint.priorityRuntime,
              turnRunnerState: 'end_step_priority', consecutivePriorityPasses: 0 };
            await engineCall('importSyncCheckpoint', checkpoint);
          }
          state = await call('uiState'); refs.stateRef.current = state;
          for (let index = 0; index < 8 && state.decision.kind === 'priority'; index++) {
            await dispatchBoth({ type: 'priority_action', action_ref: { kind: 'pass_priority' } }, `kinship-pass-${index}`);
          }
          if (state.decision.description !== 'Look at the top card of your library') {
            throw new Error(`Missing Kinship look offer: ${JSON.stringify(state.decision)}`);
          }
          await dispatchBoth({ type: 'select_options', option_indices: [1] }, 'kinship-look');
          const revealOffer = state.decision;
          if (revealOffer.description !== 'Reveal it') throw new Error(`Missing reveal offer: ${JSON.stringify(revealOffer)}`);
          const peerBeforeReveal = await peer.call('exportSyncCheckpoint');
          const hiddenTopId = peerBeforeReveal.players[1].library.at(-1);
          if (peerBeforeReveal.objects.find(object => object.id === hiddenTopId).name !== 'Hidden Card') {
            throw new Error('Private look leaked the top card to the peer');
          }
          await dispatchBoth({ type: 'select_options', option_indices: [acceptKinship ? 1 : 0] }, 'kinship-reveal');
          const finalOwner = await call('exportSyncCheckpoint');
          const finalPeer = await peer.call('exportSyncCheckpoint');
          return { cardName, topName, acceptKinship, stage: 'kinship-complete', error: null, stages,
            normalResolution, revealOffer, finalDecision: state.decision.kind,
            wolfCount: finalOwner.objects.filter(object => object.name === 'Wolf' && object.zone === 'battlefield').length,
            peerTopName: finalPeer.objects.find(object => object.id === hiddenTopId).name,
            owner: playerSummary(finalOwner), peer: playerSummary(finalPeer),
            hashesEqual: await publicHash(call) === await publicHash(peer.call) };
        }
        let flashbackAction = null;
        if (cardName === 'Faithless Looting') {
          stage = 'flashback-setup';
          for (let index = 0; index < 3; index++) {
            await call('addCardToZone', 1, 'Mountain', 'battlefield', true);
            await peer.call('addCardToZone', 1, 'Mountain', 'battlefield', true);
          }
          state = await call('uiState'); refs.stateRef.current = state;
          const checkpoint = await call('exportSyncCheckpoint');
          const spell = checkpoint.objects.find(object => object.name === cardName && object.zone === 'graveyard');
          flashbackAction = state.decision.actions.find(action => action.action_ref?.kind === 'cast_spell' && Number(action.object_id) === spell.id);
          if (!flashbackAction) throw new Error(`No flashback action: ${JSON.stringify(state.decision)}`);
          await resolveCast(flashbackAction, 'flashback-');
        }
        stage = 'final-checkpoint';
        const own = await call('exportSyncCheckpoint');
        const other = await peer.call('exportSyncCheckpoint');
        return { cardName, stage, error: null, stages, normalResolution, flashbackAction, finalDecision: state.decision?.kind,
          owner: playerSummary(own), peer: playerSummary(other), hashesEqual: await publicHash(call) === await publicHash(peer.call) };
      } catch (error) {
        return { cardName, stage, error: error.message, stages, pendingRequirements, decision: state.decision,
          ownerCheckpoint: await call('exportSyncCheckpoint'), peerCheckpoint: await peer.call('exportSyncCheckpoint') };
      }
    } finally { reactRoot?.unmount(); worker.terminate(); peer?.worker.terminate(); }
  }, { cardName, topName, acceptKinship, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
  assert.deepEqual(pageErrors, []);
  if (process.env.ZIFFLE_PROBE_VERBOSE) console.log(JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ cardName, stage: result.stage, error: result.error,
    stages: result.stages.map(stage => ({ label: stage.label, privateCount: stage.privateCount,
      decision: stage.decision.kind, peerCandidates: stage.peerDecision.candidates?.map(card => card.name),
      hashesEqual: stage.hashesEqual, publicOpenings: stage.openings.map(opening => opening.card) })),
    normalResolution: result.normalResolution, flashbackAction: result.flashbackAction, owner: result.owner, peer: result.peer }));
  assert.equal(result.error, null, `${cardName} failed at ${result.stage}`);
  assert.equal(result.finalDecision, 'priority');
  assert.equal(result.hashesEqual, true, 'both peers agree after public opening application');
  if (topName) {
    assert.ok(result.stages.every(stage => stage.hashesEqual), 'Kinship stays synchronized at every decision');
    assert.equal(result.revealOffer.options.find(option => option.index === 1).legal, topName === 'Llanowar Elves');
    const look = result.stages.find(stage => stage.label === 'kinship-look');
    assert.equal(look.privateCount, 1);
    assert.equal(look.peerDecision.description, 'Reveal it');
    assert.equal(look.openings.length, 0, 'looking reveals nothing publicly');
    const reveal = result.stages.find(stage => stage.label === 'kinship-reveal');
    assert.equal(result.wolfCount, Number(acceptKinship));
    if (acceptKinship) {
      assert.equal(result.peerTopName, topName);
      assert.ok(reveal.openings.some(opening => opening.card === topName && opening.proof && opening.salted));
    } else {
      assert.equal(result.peerTopName, 'Hidden Card');
      assert.equal(reveal.openings.length, 0, 'declining preserves the private identity');
      assert.ok(!reveal.requirements.some(requirement => requirement.type === 'public_open'));
    }
    return;
  }
  if (['Spelunking', 'Collected Company'].includes(cardName)) {
    assert.equal(result.owner.hand, 0);
    assert.equal(result.owner.library, cardName === 'Spelunking' ? 59 : 60);
    assert.ok(result.stages.every(stage => stage.hashesEqual), 'public hashes match at every suspended instruction');
    const resolving = result.stages.find(stage => stage.label === (cardName === 'Spelunking' ? 'resolve-pass-3' : 'resolve-pass-1'));
    assert.equal(resolving.privateCount, cardName === 'Spelunking' ? 1 : 6);
    assert.equal(resolving.decision.kind, cardName === 'Spelunking' ? 'select_options' : 'select_objects');
    if (cardName === 'Spelunking') {
      const landChoice = result.stages.find(stage => stage.label.startsWith('ordering-'));
      assert.equal(landChoice.decision.kind, 'select_objects');
      assert.ok(landChoice.decision.candidates.every(card => card.name === 'Island'));
      assert.ok(landChoice.peerDecision.candidates.every(card => card.name === 'Hidden card'));
      assert.ok(result.stages.flatMap(stage => stage.openings).some(opening => opening.card === 'Island'),
        'putting the privately drawn land onto the battlefield requires its public proof');
    } else {
      assert.equal(resolving.decision.candidates.length, 0, 'the owner knows none of the six cards is a creature');
      assert.equal(resolving.peerDecision.candidates.length, 6, 'the peer retains the same explicit private choice');
    }
    assert.ok(result.stages.flatMap(stage => stage.openings).every(opening => opening.proof && opening.salted));
    return;
  }
  assert.equal(result.owner.hand, cardName === 'Brainstorm' ? 1 : 0);
  assert.equal(result.owner.library, cardName === 'Brainstorm' ? 59 : 56);
  assert.equal(result.owner.graveyard.length, cardName === 'Brainstorm' ? 1 : 4);
  assert.ok(result.normalResolution.owner.graveyard.includes(cardName));
  assert.equal(result.normalResolution.hashesEqual, true);
  assert.ok(result.stages.every(stage => stage.hashesEqual), 'public hashes match at every nested prompt');
  const drawn = result.stages.find(stage => stage.label === 'resolve-pass-1');
  assert.equal(drawn.privateCount, cardName === 'Brainstorm' ? 3 : 2);
  assert.ok(drawn.decision.candidates.every(card => card.name === 'Island'));
  assert.ok(drawn.peerDecision.candidates.every(card => card.name === 'Hidden card'), 'private draws stay hidden from the peer');
  assert.ok(result.stages.flatMap(stage => stage.openings).every(opening => opening.proof && opening.salted));
  if (cardName === 'Brainstorm') {
    assert.deepEqual(result.peer.privateCards, ['Hidden Card']);
    assert.equal(result.stages.find(stage => stage.label === 'hand-choice-0').decision.kind, 'select_options');
    assert.ok(result.stages.flatMap(stage => stage.openings).every(opening => opening.card === 'Brainstorm'), 'returning cards does not publicly reveal them');
  } else {
    assert.deepEqual(result.normalResolution.peer.graveyard.sort(), ['Faithless Looting', 'Island', 'Island']);
    assert.deepEqual(result.owner.exile, ['Faithless Looting']);
    assert.deepEqual(result.peer.exile, ['Faithless Looting']);
    assert.deepEqual(result.peer.graveyard, Array(4).fill('Island'));
    const flashbackDrawn = result.stages.find(stage => stage.label === 'flashback-resolve-pass-1');
    assert.equal(flashbackDrawn.privateCount, 2);
    assert.ok(flashbackDrawn.peerDecision.candidates.every(card => card.name === 'Hidden card'));
  }
});
