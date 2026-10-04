import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

for (const { restInPeace, knownControl } of [
  { restInPeace: false, knownControl: true },
  { restInPeace: true, knownControl: false },
]) {
  const cardName = 'Tome Scour';
  test(`${cardName} reveals only the mixed top five when moving to ${restInPeace ? 'exile under Rest in Peace' : 'graveyard'}`, { timeout: 120000 }, async t => {
    const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
      server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await server.listen(); t.after(() => server.close());
    const browser = await chromium.launch(); t.after(() => browser.close());
    const page = await browser.newPage();
    const pageErrors = [];
    page.on('pageerror', error => pageErrors.push(error.message));
    await page.route('**/mill-edge-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root"></div>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/mill-edge-test`);
    const result = await page.evaluate(async ({ verifierUrl, cardName, restInPeace, knownControl }) => {
      const { mountOpeningServices } = await import('/tests/ziffle-public-opening-zone-change-harness.js');
      const { buildPrivateDeckManifest, publicDeckManifest, publicCheckpointHash, authorizeCryptoMaterialRequestRequirements } = await import('/src/lib/multiplayer-audit.js');
      const { buildZiffleRuntimeManifest } = await import('/src/lib/ziffle-runtime-manifest.js');
      const { mergeAuditOpenings } = await import('/src/hooks/peer-lobby/shared.js');
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
        const matchId = `mill-edge:${cardName}`;
        const deck = Array(61).fill('Mountain');
        deck[4] = cardName;
        deck[5] = 'Grizzly Bears';
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
        const expectedCards = ['Grizzly Bears', 'Island', 'Lightning Bolt', 'Forest', 'Savannah Lions'];
        expectedCards.forEach((name, index) => { deck[reveals.find(reveal => reveal.cardPosition === 60 - index).originalSlot] = name; });
        const manifests = await Promise.all([0, 1].map(seat => buildPrivateDeckManifest({ matchId, owner: seat, deck })));
        const ceremony = { owner: 1, deckCount, context, keys, steps, deckHash: verified.deckHash, tokens, reveals };
        peer = createWorkerSession(); await peer.ready;
        await owner.call('setPerspective', 1);
        await peer.call('setPerspective', 0);
        const setupCall = async (method, ...args) => {
          const result = await owner.call(method, ...args);
          await peer.call(method, ...args);
          return result;
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
        if (!source) throw new Error('Missing committed spell in library');
        const secret = manifests[1].slotSecrets.find(entry => entry.slot === 4);
        await owner.call('revealHiddenPosition', { owner: 1, objectId: source.id, position,
          originalSlot: 4, cardName: secret.card, commitment: secret.commitment,
          positionCommitment: `ziffle:${ceremony.deckHash}:${position}` });
        // Both engines draw the same committed cards. The hand spell remains
        // concealed on the receiving peer until its verified public opening.
        for (let index = deckCount - 1; index >= position; index--) await setupCall('drawCard', 1);
        await setupCall('addCardToZone', 1, 'Island', 'battlefield', true);
        if (restInPeace) await setupCall('addCardToZone', 1, 'Rest in Peace', 'battlefield', true);
        state = await owner.call('uiState');
        const fixture = await owner.call('getHiddenCardState');
        const spell = fixture.objects.find(object => object.stableId === source.stableId);
        // The mixed case starts with only the top card privately known to its
        // owner (seat 0). The acting peer still sees five encrypted cards.
        if (knownControl) {
          for (const session of [peer]) {
            const checkpoint = await session.call('getHiddenCardState');
            for (const id of checkpoint.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library').sort((left, right) => (left.hiddenCard.publicSlot ?? left.hiddenCard.slot) - (right.hiddenCard.publicSlot ?? right.hiddenCard.slot)).slice(-1).map(object => object.id)) {
              const object = checkpoint.objects.find(entry => entry.id === id);
              const position = object.hiddenCard.publicSlot ?? object.hiddenCard.slot;
              const originalSlot = reveals.find(reveal => reveal.cardPosition === position).originalSlot;
              const secret = manifests[0].slotSecrets.find(entry => entry.slot === originalSlot);
              await session.call('revealHiddenPosition', { owner: 0, objectId: id, position,
                originalSlot, cardName: secret.card, commitment: secret.commitment,
                positionCommitment: `ziffle:${ceremony.deckHash}:${position}` });
            }
          }
        }
        const peerBefore = await peer.call('getHiddenCardState');
        const actorBefore = await owner.call('getHiddenCardState');
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
        if (knownControl) {
          // A private look hydrates the owner's engine and retains its proved
          // opening in the owner's local cache. Reconstruct that fixture state
          // without sharing the card with the acting peer.
          const known = peerBefore.objects.find(object => object.hiddenCard?.owner === 0 && object.zone === 'library' && object.name !== 'Hidden Card');
          refs.gameRef.current = wrap(peer); refs.multiplayerRef.current.localPlayerIndex = 0;
          refs.privateDeckManifestsRef.current = new Map([[`${matchId}:0`, manifests[0]]]);
          const exported = await peer.call('exportHiddenCardOpening', BigInt(known.id));
          await services.current.buildLocalOpeningFromRequirement({ type: 'private_open', owner: 0, viewer: 0,
            objectId: known.id, slot: known.hiddenCard.slot, commitment: known.hiddenCard.commitment,
            publicSlot: known.hiddenCard.publicSlot, publicCommitment: known.hiddenCard.publicCommitment }, exported,
          { forceZiffleOpeningProof: true });
          refs.gameRef.current = wrap(owner); refs.multiplayerRef.current.localPlayerIndex = 1;
          refs.privateDeckManifestsRef.current = new Map([[`${matchId}:1`, manifests[1]]]);
        }
        const buildOpenings = async (command, requirements, peerRequirements = requirements) => {
          const openings = [];
          for (const seat of [0, 1]) {
            const session = seat === 1 ? owner : peer;
            refs.gameRef.current = wrap(session);
            refs.stateRef.current = await session.call('uiState');
            refs.multiplayerRef.current.localPlayerIndex = seat;
            refs.privateDeckManifestsRef.current = new Map([[`${matchId}:${seat}`, manifests[seat]]]);
            const localRequirements = seat === 0 ? peerRequirements : requirements;
            if (command) openings.push(...await services.current.buildLocalOpeningsForCommand(command, localRequirements));
            openings.push(...await services.current.buildLocalRequirementOpeningsForRequirements(localRequirements, { forceZiffleOpeningProof: true }));
          }
          refs.gameRef.current = wrap(owner); refs.stateRef.current = state;
          refs.multiplayerRef.current.localPlayerIndex = 1;
          refs.privateDeckManifestsRef.current = new Map([[`${matchId}:1`, manifests[1]]]);
          return mergeAuditOpenings(openings);
        };
        const applyPeer = async (openings, timing, command = null) => {
          refs.gameRef.current = wrap(peer); refs.multiplayerRef.current.localPlayerIndex = 0;
          refs.stateRef.current = await peer.call('uiState');
          try { await services.current.revealAuditOpenings(openings, { timing, command, updateState: false, previewInspector: false }); }
          finally { refs.gameRef.current = wrap(owner); refs.multiplayerRef.current.localPlayerIndex = 1; refs.stateRef.current = state; }
        };
        const dispatchBoth = async command => {
          stage = `${trace.length}:${command.type}:command-opening`;
          // Receiver validation applies the spell/selection's command opening
          // before asking its engine to preview that command's consequences.
          const commandOpenings = await buildOpenings(command, []);
          await applyPeer(commandOpenings, 'pre', command);
          await services.current.revealAuditOpenings(commandOpenings, { timing: 'pre', command, updateState: false, previewInspector: false });
          stage = `${trace.length}:${command.type}:preview`;
          const requirements = await owner.call('previewCryptoRequirements', command);
          const peerRequirements = await peer.call('previewCryptoRequirements', command);
          const declaration = requirement => ({ id: requirement.id, owner: requirement.owner, objectId: requirement.objectId ?? requirement.object_id,
            slot: requirement.slot, commitment: requirement.commitment,
            timing: requirement.timing || 'post', from: requirement.from, to: requirement.to });
          const publicMoveDeclarations = [requirements, peerRequirements].map(list => list
            .filter(requirement => requirement.type === 'public_open' && requirement.owner === 0 && requirement.from)
            .map(declaration).sort((left, right) => left.objectId - right.objectId));
          if (JSON.stringify(publicMoveDeclarations[0]) !== JSON.stringify(publicMoveDeclarations[1])) {
            throw new Error(`Public move authorization depends on private card knowledge: ${JSON.stringify(publicMoveDeclarations)}`);
          }
          const authorizedPeerRequirements = authorizeCryptoMaterialRequestRequirements({ localSeat: 0,
            requestedRequirements: requirements, previewedRequirements: peerRequirements });
          const prepared = mergeAuditOpenings(commandOpenings, await buildOpenings(command, requirements, authorizedPeerRequirements));
          const publicPreRequirements = requirements.filter(requirement => requirement.type === 'public_open' && requirement.timing === 'pre');
          const preRequirementChecks = { checked: publicPreRequirements.length, lateOpeningRejected: false };
          if (publicPreRequirements.length) {
            await services.current.verifyAuditSatisfiesCryptoRequirements({ requirements: publicPreRequirements, audit: { openings: prepared } });
            try {
              await services.current.verifyAuditSatisfiesCryptoRequirements({ requirements: publicPreRequirements,
                audit: { openings: prepared.map(opening => ({ ...opening, timing: 'post' })) } });
            } catch (error) {
              if (!error.message.includes('Public opening must be applied before the command')) throw error;
              preRequirementChecks.lateOpeningRejected = true;
            }
          }
          stage = `${trace.length}:${command.type}:pre-opening`;
          await applyPeer(prepared, 'pre', command);
          await services.current.revealAuditOpenings(prepared, { timing: 'pre', command, updateState: false, previewInspector: false });
          const sourceStates = await Promise.all([owner, peer].map(session => session.call('getHiddenCardState')));
          const preHydratedPublicMoves = publicPreRequirements.filter(requirement => requirement.owner === 0).map(requirement => {
            const id = requirement.objectId ?? requirement.object_id;
            return { requirementId: requirement.id, id, sources: sourceStates.map(checkpoint => {
              const object = checkpoint.objects.find(item => item.id === id);
              return object ? { name: object.name, originalCardName: object.originalCardName, zone: object.zone } : null;
            }) };
          });
          stage = `${trace.length}:${command.type}:dispatch`;
          state = await owner.call('dispatch', command); refs.stateRef.current = state;
          const peerState = await peer.call('dispatch', command);
          if (state.decision?.kind !== peerState.decision?.kind) throw new Error('Peer decisions differ');
          stage = `${trace.length}:${command.type}:post-opening`;
          await applyPeer(prepared, 'post');
          await services.current.revealAuditOpenings(prepared, { timing: 'post', updateState: false, previewInspector: false });
          const afterRequirements = state.crypto_requirements || state.cryptoRequirements || [];
          const afterOpenings = await buildOpenings(null, [...requirements, ...afterRequirements]);
          await applyPeer(afterOpenings, 'post');
          await services.current.revealAuditOpenings(afterOpenings, { timing: 'post', updateState: false, previewInspector: false });
          stage = `${trace.length}:${command.type}:public-hash`;
          const ownHash = await publicCheckpointHash(await owner.call('exportPublicAuditCheckpoint'));
          const peerHash = await publicCheckpointHash(await peer.call('exportPublicAuditCheckpoint'));
          if (ownHash !== peerHash) throw new Error(`Public checkpoint hashes differ: ${ownHash} / ${peerHash}`);
          trace.push({ command, publicMoveDeclarations, preRequirementChecks, preHydratedPublicMoves, decision: state.decision?.kind, ordering: state.decision?.reason === 'Ordering' ? state.decision.options.map(option => option.description) : null, prepared: prepared.map(opening => ({ owner: opening.owner, slot: opening.slot, position: opening.position, origin: opening.originPosition, card: opening.card, timing: opening.timing })),
            requirements: [...requirements, ...afterRequirements].map(requirement => ({ type: requirement.type, owner: requirement.owner, slot: requirement.slot, from: requirement.from, to: requirement.to })) });
        };
        stage = 'find-cast-action';
        const action = state.decision?.actions?.find(item => item.action_ref?.kind === 'cast_spell' && Number(item.object_id) === spell.id);
        if (!action) throw new Error(`Card not playable: ${JSON.stringify(state.decision)}`);
        await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: spell.id });
        let after;
        for (let index = 0; index < 12; index++) {
          after = await owner.call('getHiddenCardState');
          const currentSpell = after.objects.find(object => object.stableId === spell.stableId);
          if (state.decision?.kind === 'priority' && currentSpell && !['hand', 'stack'].includes(currentSpell.zone)) break;
          const decision = state.decision;
          if (decision?.kind === 'targets') {
            await dispatchBoth({ type: 'select_targets', targets: [{ kind: 'player', player: 0 }] });
          } else if (decision?.kind === 'mana_payment') {
            await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
          } else if (decision?.kind === 'select_options') {
            await dispatchBoth({ type: 'select_options', option_indices: decision.options.filter(option => option.legal).map(option => option.index) });
          } else if (decision?.kind === 'priority') {
            const pass = decision.actions.find(item => item.action_ref?.kind === 'pass_priority');
            if (!pass) throw new Error('No pass action');
            await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
          } else throw new Error(`Unexpected decision ${JSON.stringify(decision)}`);
        }
        after = await owner.call('getHiddenCardState');
        const peerAfter = await peer.call('getHiddenCardState');
        const destination = restInPeace ? 'exile' : 'graveyard';
        const summarize = object => ({ id: object.id, stableId: object.stableId, owner: object.hiddenCard?.owner,
          zone: object.zone, name: object.name, originalCardName: object.originalCardName, hiddenCard: object.hiddenCard });
        const publicCards = after.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === destination);
        const peerPublicCards = peerAfter.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === destination);
        const library = after.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library');
        const peerLibrary = peerAfter.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library');
        return { restInPeace, knownControl, trace, expectedCards, destination,
          initialActorKnownLibraryCards: actorBefore.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library' && object.name !== 'Hidden Card').length,
          initialKnownLibraryCards: peerBefore.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library' && object.name !== 'Hidden Card').length,
          publicCards: publicCards.map(summarize), peerPublicCards: peerPublicCards.map(summarize),
          graveyardIds: after.players[0].graveyard, peerGraveyardIds: peerAfter.players[0].graveyard,
          libraryCount: library.length, peerLibraryCount: peerLibrary.length,
          libraryStillPrivate: library.every(object => object.name === 'Hidden Card' && object.originalCardName == null),
          peerLibraryStillPrivate: peerLibrary.every(object => object.name === 'Hidden Card' && object.originalCardName == null),
          peerBeforeSpell: peerBefore.objects.find(object => object.stableId === spell.stableId),
          afterSpell: after.objects.find(object => object.stableId === spell.stableId),
          decision: state.decision?.kind };
      } catch (error) {
        throw new Error(`${cardName} failed at ${stage}: ${error.message}; trace=${JSON.stringify(trace)}`);
      } finally { reactRoot?.unmount(); owner.worker.terminate(); peer?.worker.terminate(); }
    }, { cardName, restInPeace, knownControl, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
    assert.deepEqual(pageErrors, []);
    const reportDir = path.resolve(root, '../../reports/player-mcp/zkp-protocol-fixes-2026-09-27');
    await mkdir(reportDir, { recursive: true });
    await writeFile(path.join(reportDir, `mill-mixed-${restInPeace ? 'exile' : 'graveyard'}-security-evidence.json`), JSON.stringify(result, null, 2) + '\n');
    t.diagnostic(JSON.stringify({ restInPeace, knownControl, destination: result.destination, actions: result.trace.length,
      publicNames: result.publicCards.map(card => card.name), libraryStillPrivate: result.libraryStillPrivate,
      preparedPublicOpenings: result.trace.flatMap(step => step.prepared).filter(opening => opening.owner === 0) }));
    assert.equal(result.peerBeforeSpell.name, 'Hidden Card');
    assert.equal(result.decision, 'priority');
    assert.equal(result.afterSpell.zone, restInPeace ? 'exile' : 'graveyard');
    assert.equal(result.initialActorKnownLibraryCards, 0, 'the acting peer does not share the owner’s privately known top card');
    assert.equal(result.initialKnownLibraryCards, knownControl ? 1 : 0);
    assert.equal(result.publicCards.length, 5);
    assert.deepEqual(result.publicCards.map(card => card.name).sort(), [...result.expectedCards].sort());
    assert.deepEqual(result.peerPublicCards, result.publicCards);
    assert.deepEqual(result.peerGraveyardIds, result.graveyardIds, 'both peers preserve the same graveyard ordering');
    assert.equal(result.graveyardIds.length, restInPeace ? 0 : 5);
    assert.equal(result.libraryCount, 56);
    assert.equal(result.peerLibraryCount, 56);
    assert.equal(result.libraryStillPrivate, true, 'unmilled library identities are not opened');
    assert.equal(result.peerLibraryStillPrivate, true);
    const preChecks = result.trace.map(step => step.preRequirementChecks).filter(check => check.checked > 0);
    assert.ok(preChecks.length > 0, 'public transitions require identity hydration before dispatch');
    assert.ok(preChecks.every(check => check.lateOpeningRejected), 'a post-timed opening cannot satisfy a pre-dispatch public move requirement');
    const preHydrated = result.trace.flatMap(step => step.preHydratedPublicMoves).filter(move => move.sources[0]?.zone === 'library');
    assert.equal(new Set(preHydrated.map(move => move.id)).size, 5);
    assert.deepEqual([...new Set(preHydrated.map(move => move.sources[0].name))].sort(), [...result.expectedCards].sort());
    for (const move of preHydrated) {
      assert.deepEqual(move.sources[1], move.sources[0], 'both peers have the same printed identity before applying the move');
      assert.equal(move.sources[0].originalCardName, move.sources[0].name);
    }
    const prepared = result.trace.flatMap(step => step.prepared).filter(opening => opening.owner === 0);
    assert.equal(new Set(prepared.map(opening => opening.position)).size, 5, 'all five public identities are prepared from preview requirements before the move command is dispatched');
    assert.deepEqual([...new Set(prepared.map(opening => opening.card))].sort(), [...result.expectedCards].sort());
    for (const card of result.publicCards) {
      assert.ok(card.hiddenCard.originCommitment.startsWith('ziffle:'));
      assert.equal(card.hiddenCard.originSlot, card.hiddenCard.publicSlot);
    }
  });
}
