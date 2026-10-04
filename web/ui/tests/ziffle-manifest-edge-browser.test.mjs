import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

for (const cardName of ['Den Protector', 'Rattleclaw Mystic']) {
  test(`${cardName} is manifested by Scroll of Fate then turns face up across two committed peers`, { timeout: 120000 }, async t => {
    const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
      server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await server.listen(); t.after(() => server.close());
    const browser = await chromium.launch(); t.after(() => browser.close());
    const page = await browser.newPage();
    const pageErrors = [];
    page.on('pageerror', error => pageErrors.push(error.message));
    await page.route('**/manifest-edge-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root"></div>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/manifest-edge-test`);
    const result = await page.evaluate(async ({ verifierUrl, cardName }) => {
      const { mountOpeningServices } = await import('/tests/ziffle-public-opening-zone-change-harness.js');
      const { buildPrivateDeckManifest, publicDeckManifest, publicCheckpointHash } = await import('/src/lib/multiplayer-audit.js');
      const { buildZiffleRuntimeManifest } = await import('/src/lib/ziffle-runtime-manifest.js');
      const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
      const api = await import(verifierUrl);
      await api.default();
      function createWorkerSession() {
        const decoder = createSnapshotDecoder(), pending = new Map(), analyses = new Map(), waiters = new Map();
        const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
        let requestId = 0;
        const ready = new Promise((resolve, reject) => {
          worker.onerror = error => reject(new Error(error.message));
          worker.onmessage = ({ data }) => {
            if (data.type === 'error') return reject(new Error(data.error.stack || data.error.message));
            if (data.type === 'ready') return resolve();
            if (data.type === 'priorityAnalysis') { if (data.decision?.analysis_complete !== true) return; analyses.set(data.revision, data.decision); waiters.get(data.revision)?.(data.decision); return; }
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
          const decision = analyses.get(revision) || await new Promise(resolve => waiters.set(revision, resolve));
          waiters.delete(revision); return { ...value, decision };
        });
        worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
        return { worker, call, ready };
      }
      const owner = createWorkerSession();
      let peer, reactRoot, stage = 'fixture';
      const trace = [];
      try {
        await owner.ready;
        const matchId = `manifest-edge:${cardName}`;
        const deck = Array(61).fill('Mountain');
        deck[4] = cardName;
        deck[5] = 'Grizzly Bears';
        const manifests = await Promise.all([0, 1].map(seat => buildPrivateDeckManifest({ matchId, owner: seat, deck })));
        const deckCount = deck.length, context = matchId;
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
        const ceremony = { owner: 1, deckCount, context, keys, steps, deckHash: verified.deckHash, tokens, reveals };
        peer = createWorkerSession(); await peer.ready;
        await owner.call('setPerspective', 1);
        await peer.call('setPerspective', 0);
        const setupCall = async (method, ...args) => {
          const result = await owner.call(method, ...args);
          await peer.call(method, ...args);
          return result;
        };
        const readFixture = async session => {
          const metadata = await session.call('getHiddenCardState');
          const ui = await session.call('uiState');
          const audit = await session.call('exportPublicAuditCheckpoint');
          return { ...metadata, stack: ui.stack_objects, objects: metadata.objects.map(object => ({
            ...object, faceDown: audit.objects.find(card => card.id === object.id)?.faceDown ?? false,
          })) };
        };
        let state = await setupCall('startMatch', { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1,
          format: 'normal', startingPlayer: 1, openingHandSize: 0, decks: [[], []], publicDecklists: [deck, deck],
          hiddenDeckManifests: manifests.map((manifest, seat) => buildZiffleRuntimeManifest(manifest, { ...ceremony, owner: seat })) });
        for (let index = 0; index < 30 && state.phase !== 'first main phase'; index++) {
          const action = state.decision?.actions?.find(item => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(item.action_ref?.kind));
          if (!action) throw new Error(`Unexpected pregame decision ${JSON.stringify(state.decision)}`);
          state = await setupCall('dispatch', { type: 'priority_action', action_ref: action.action_ref });
        }
        if (state.phase !== 'first main phase') throw new Error('Fixture did not reach main phase');
        const position = reveals.find(reveal => reveal.originalSlot === 4).cardPosition;
        const initial = await owner.call('getHiddenCardState');
        const source = initial.objects.find(object => object.hiddenCard?.owner === 1 && object.hiddenCard.slot === position);
        if (!source) throw new Error('Missing committed hand fixture');
        const secret = manifests[1].slotSecrets.find(entry => entry.slot === 4);
        await owner.call('revealHiddenPosition', { owner: 1, objectId: source.id, position,
          originalSlot: 4, cardName: secret.card, commitment: secret.commitment,
          positionCommitment: `ziffle:${ceremony.deckHash}:${position}` });
        for (let index = deckCount - 1; index >= position; index--) await setupCall('drawCard', 1);
        const creatureId = Number(await setupCall('addCardToZone', 1, 'Grizzly Bears', 'graveyard', true));
        const scrollId = Number(await setupCall('addCardToZone', 1, 'Scroll of Fate', 'battlefield', true));
        for (let index = 0; index < 8; index++) await setupCall('addCardToZone', 1, 'Forest', 'battlefield', true);
        state = await owner.call('uiState');
        const fixture = await readFixture(owner);
        const spell = fixture.objects.find(object => object.stableId === source.stableId);
        const creature = fixture.objects.find(object => object.id === creatureId);
        const peerBefore = await readFixture(peer);
        const wrap = session => new Proxy({}, { get: (_, method) => method.startsWith('ziffle')
          ? async input => api[method](input) : (...args) => session.call(method, ...args) });
        const refs = { gameRef: { current: wrap(owner) }, stateRef: { current: state },
          multiplayerRef: { current: { localPlayerIndex: 1, players: manifests.map((manifest, index) => ({ index, deckAuditManifest: publicDeckManifest(manifest) })) } },
          verifiedAuditOpeningsRef: { current: new Set() },
          matchStartPayloadRef: { current: { auditMatchId: matchId, deckAuditManifests: manifests.map(publicDeckManifest), players: [{ index: 0 }, { index: 1 }] } },
          privateDeckManifestsRef: { current: new Map(manifests.map((manifest, index) => [`${matchId}:${index}`, manifest])) },
          liveZiffleCeremoniesRef: { current: new Map([0, 1].map(seat => [seat, { ...ceremony, owner: seat }])) },
          localZiffleCeremonyLookupRef: { current: new Map([['initial', ceremony]]) },
          setState() {}, setStatus() {}, setMultiplayer() {} };
        const base = new Proxy(refs, { get: (object, key) => object[key] ||= { current: new Map() } });
        const services = { current: { collectZiffleRevealTokens: async (_selected, position) => tokens.filter(token => token.cardPosition === position),
          collectZiffleRevealTokensBatch: async (_selected, selected) => tokens.filter(token => selected.includes(token.cardPosition)),
          commandObjectHiddenRefs: () => [], commandObjectStableIds: () => [],
          currentObjectIdForHiddenRef: async () => null, currentObjectIdForStableId: async () => null } };
        reactRoot = mountOpeningServices(base, services);
        const buildOpenings = async (command, requirements) => {
          const openings = [];
          for (const seat of [0, 1]) {
            refs.multiplayerRef.current.localPlayerIndex = seat;
            if (command) openings.push(...await services.current.buildLocalOpeningsForCommand(command, requirements));
            openings.push(...await services.current.buildLocalRequirementOpeningsForRequirements(requirements, { forceZiffleOpeningProof: true }));
          }
          refs.multiplayerRef.current.localPlayerIndex = 1;
          return [...new Map(openings.map(opening => [`${opening.owner}:${opening.slot}:${opening.commitment}:${opening.timing || 'pre'}`, opening])).values()];
        };
        const applyPeer = async (openings, timing, command = null) => {
          refs.gameRef.current = wrap(peer); refs.multiplayerRef.current.localPlayerIndex = 0;
          refs.stateRef.current = await peer.call('uiState');
          try { await services.current.revealAuditOpenings(openings, { timing, command, updateState: false, previewInspector: false }); }
          finally { refs.gameRef.current = wrap(owner); refs.multiplayerRef.current.localPlayerIndex = 1; refs.stateRef.current = state; }
        };
        const dispatchBoth = async command => {
          stage = `${trace.length}:${command.type}:preview`;
          const requirements = await owner.call('previewCryptoRequirements', command);
          const prepared = await buildOpenings(command, requirements);
          stage = `${trace.length}:${command.type}:pre-opening`;
          await applyPeer(prepared, 'pre', command);
          stage = `${trace.length}:${command.type}:dispatch`;
          state = await owner.call('dispatch', command); refs.stateRef.current = state;
          const peerState = await peer.call('dispatch', command);
          if (state.decision?.kind !== peerState.decision?.kind) throw new Error('Peer decisions differ');
          stage = `${trace.length}:${command.type}:post-opening`;
          await applyPeer(prepared, 'post');
          const afterRequirements = state.crypto_requirements || state.cryptoRequirements || [];
          const afterOpenings = await buildOpenings(null, [...requirements, ...afterRequirements]);
          await applyPeer(afterOpenings, 'post');
          stage = `${trace.length}:${command.type}:public-hash`;
          const ownHash = await publicCheckpointHash(await owner.call('exportPublicAuditCheckpoint'));
          const peerHash = await publicCheckpointHash(await peer.call('exportPublicAuditCheckpoint'));
          if (ownHash !== peerHash) throw new Error(`Public checkpoint hashes differ: ${ownHash} / ${peerHash}`);
          trace.push({ command, decision: state.decision?.kind, prepared: prepared.map(opening => ({ owner: opening.owner, slot: opening.slot, origin: opening.originPosition })),
            requirements: [...requirements, ...afterRequirements].map(requirement => ({ type: requirement.type, owner: requirement.owner, slot: requirement.slot, from: requirement.from, to: requirement.to })) });
        };
        stage = 'find-cast-action';
        const action = state.decision?.actions?.find(item => item.action_ref?.kind === 'activate_ability' && Number(item.object_id) === Number(scrollId));
        if (!action) throw new Error(`Card not playable: ${JSON.stringify(state.decision)}`);
        await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: scrollId });
        let after;
        for (let index = 0; index < 12; index++) {
          after = await readFixture(owner);
          const currentSpell = after.objects.find(object => object.stableId === spell.stableId);
          if (state.decision?.kind === 'priority' && currentSpell && !['hand', 'stack'].includes(currentSpell.zone)) break;
          const decision = state.decision;
          if (decision?.kind === 'select_objects') {
            await dispatchBoth({ type: 'select_objects', object_ids: [spell.id] });
          } else if (decision?.kind === 'targets') {
            await dispatchBoth({ type: 'select_targets', targets: [{ kind: 'object', object: creature.id }] });
          } else if (decision?.kind === 'mana_payment') {
            await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
          } else if (decision?.kind === 'priority') {
            const pass = decision.actions.find(item => item.action_ref?.kind === 'pass_priority');
            if (!pass) throw new Error('No pass action');
            await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
          } else throw new Error(`Unexpected decision ${JSON.stringify(decision)}`);
        }
        const firstResolution = await readFixture(owner);
        const facedownPeer = await readFixture(peer);
        const facedownObject = firstResolution.objects.find(object => object.stableId === spell.stableId);
        if (!facedownObject.faceDown) throw new Error('Scroll of Fate did not manifest the hand card');
        stage = 'find-turn-face-up';
        const flip = state.decision?.actions?.find(item => item.action_ref?.kind === 'turn_face_up'
          && Number(item.object_id) === facedownObject.id);
        if (!flip) throw new Error(`No face-up action ${JSON.stringify(state.decision)}`);
        await dispatchBoth({ type: 'priority_action', action_ref: flip.action_ref, object_id: facedownObject.id });
        for (let index = 0; index < 12; index++) {
          const decision = state.decision;
          const checkpoint = await readFixture(owner);
          if (decision?.kind === 'priority' && checkpoint.stack.length === 0) break;
          if (decision?.kind === 'priority') {
            const pass = decision.actions.find(item => item.action_ref?.kind === 'pass_priority');
            await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
          } else if (decision?.kind === 'select_objects') {
            await dispatchBoth({ type: 'select_objects', object_ids: [creature.id] });
          } else if (decision?.kind === 'targets') {
            await dispatchBoth({ type: 'select_targets', targets: [{ kind: 'object', object: creature.id }] });
          } else if (decision?.kind === 'mana_payment') {
            await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
          } else throw new Error(`Unexpected flip decision ${JSON.stringify(decision)}`);
        }
        after = await readFixture(owner);
        const peerAfter = await readFixture(peer);
        return { cardName, trace, facedownObject, facedownPeerObject: facedownPeer.objects.find(object => object.stableId === spell.stableId), beforeSpell: spell, beforeCreature: creature,
          peerBeforeSpell: peerBefore.objects.find(object => object.stableId === spell.stableId),
          firstSpell: firstResolution.objects.find(object => object.stableId === spell.stableId),
          firstCreature: firstResolution.objects.find(object => object.stableId === creature.stableId),
          afterSpell: after.objects.find(object => object.stableId === spell.stableId),
          afterCreature: after.objects.find(object => object.stableId === creature.stableId),
          peerSpell: peerAfter.objects.find(object => object.stableId === spell.stableId),
          peerCreature: peerAfter.objects.find(object => object.stableId === creature.stableId),
          life: state.players.map(player => player.life), decision: state.decision?.kind };
      } catch (error) {
        throw new Error(`${cardName} failed at ${stage}: ${error.message}; trace=${JSON.stringify(trace)}`);
      } finally { reactRoot?.unmount(); owner.worker.terminate(); peer?.worker.terminate(); }
    }, { cardName, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
    assert.deepEqual(pageErrors, []);
    assert.equal(result.peerBeforeSpell.name, 'Hidden Card', 'the receiving peer learns the hand spell from its verified opening');
    assert.equal(result.decision, 'priority');
    assert.equal(result.facedownObject.faceDown, true);
    assert.equal(result.facedownPeerObject.faceDown, true);
    assert.equal(result.facedownPeerObject.originalCardName ?? null, null, 'face-down identity stays private until turned up');
    assert.equal(result.afterSpell.zone, 'battlefield');
    assert.equal(result.afterSpell.name, cardName);
    assert.equal(result.afterSpell.faceDown, false);
    assert.deepEqual(result.peerSpell, result.afterSpell);
    const provenance = ({ objectId, zone, ...identity }) => identity;
    assert.deepEqual(provenance(result.afterSpell.hiddenCard), provenance(result.beforeSpell.hiddenCard));
    t.diagnostic(JSON.stringify({ cardName, commands: result.trace.length, trace: result.trace }));
  });
}
