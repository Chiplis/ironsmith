import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

for (const cardName of ['Thoughtseize', 'Duress', 'Control Magic', 'Gonti, Lord of Luxury']) {
  test(`${cardName} cross-owner opening proof probe`, { timeout: 120000 }, async t => {
    const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
      server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await server.listen(); t.after(() => server.close());
    const browser = await chromium.launch(); t.after(() => browser.close());
    const page = await browser.newPage();
    const pageErrors = [];
    page.on('pageerror', error => pageErrors.push(error.message));
    await page.route('**/cross-owner-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root"></div>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/cross-owner-test`);
    const result = await page.evaluate(async ({ verifierUrl, cardName }) => {
      const { castAndResolveFixtureSpell, advanceFixtureToMain } = await import('/tests/fixtures/native-worker-game-setup.mjs');
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
        const matchId = `cross-owner:${cardName}`;
        const deck = Array(61).fill('Mountain');
        deck[4] = cardName;
        deck[5] = 'Grizzly Bears';
        deck[6] = 'Lightning Bolt';
        deck[7] = 'Island';
        const deckCount = deck.length;
        const ceremonies = [0, 1].map(seat => {
          const context = matchId;
          const entropy = seat === 0 ? ['aa', 'bb', 'cc', 'dd', 'ee', 'ff'] : ['11', '22', '33', '44', '55', '66'];
          const identities = entropy.slice(0, 2).map(byte => api.ziffleKeygen({ deckCount, context, entropyHex: byte.repeat(32) }));
          const keys = identities.map((identity, player) => ({ player, publicKeyHex: identity.publicKeyHex, ownershipProofHex: identity.ownershipProofHex }));
          const steps = [];
          for (let shuffler = 0; shuffler < 2; shuffler++) {
            const step = api.ziffleBuildShuffleStep({ deckCount, context, keys, steps, shuffler, entropyHex: entropy[shuffler + 2].repeat(32) });
            steps.push({ shuffler, deckHex: step.deckHex, proofHex: step.proofHex });
          }
          const verified = api.ziffleVerifyShuffle({ deckCount, context, keys, steps });
          const positions = Array.from({ length: deckCount }, (_, index) => index);
          const tokens = identities.flatMap((identity, index) => api.ziffleBuildRevealTokens({ deckCount, context, keys, steps,
            ...identity, cardPositions: positions, entropyHex: entropy[index + 4].repeat(32) }));
          const reveals = api.ziffleRevealCards({ deckCount, context, keys, steps, cardPositions: positions, tokens });
          return { owner: seat, deckCount, context, keys, steps, deckHash: verified.deckHash, tokens, reveals };
        });
        const isTheft = cardName === 'Control Magic';
        const isGonti = cardName === 'Gonti, Lord of Luxury';
        const decks = [deck.slice(), deck.slice()];
        const subjects = [{ seat: 1, position: deckCount - 1, name: cardName },
          ...(isTheft ? ['Grizzly Bears'] : isGonti ? ['Grizzly Bears', 'Lightning Bolt', 'Island', 'Mountain'] : ['Grizzly Bears', 'Lightning Bolt', 'Island'])
            .map((name, index) => ({ seat: 0, position: deckCount - 1 - index, name }))];
        for (const subject of subjects) {
          subject.slot = ceremonies[subject.seat].reveals.find(reveal => reveal.cardPosition === subject.position).originalSlot;
          decks[subject.seat][subject.slot] = subject.name;
        }
        const manifests = await Promise.all(decks.map((cards, seat) => buildPrivateDeckManifest({ matchId, owner: seat, deck: cards })));
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
          const audit = await session.call('exportPublicAuditCheckpoint');
          const ui = await session.call('uiState');
          return { ...metadata, stack: ui.stack_objects, objects: metadata.objects.map(object => {
            const publicObject = audit.objects.find(card => card.id === object.id);
            const exile = ui.players.flatMap(player => player.exile_cards).find(card => card.id === object.id);
            return { ...object, owner: object.hiddenCard?.owner ?? publicObject?.owner,
              controller: publicObject?.controller ?? object.hiddenCard?.owner,
              faceDown: publicObject?.faceDown ?? exile?.face_down ?? false };
          }) };
        };
        let state = await setupCall('startMatch', { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1,
          format: 'normal', startingPlayer: isTheft ? 0 : 1, openingHandSize: 0, decks: [[], []], publicDecklists: decks,
          hiddenDeckManifests: manifests.map((manifest, seat) => buildZiffleRuntimeManifest(manifest, ceremonies[seat])) });
        for (let index = 0; index < 30 && state.phase !== 'first main phase'; index++) {
          const action = state.decision?.actions?.find(item => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(item.action_ref?.kind));
          if (!action) throw new Error(`Unexpected pregame decision ${JSON.stringify(state.decision)}`);
          state = await setupCall('dispatch', { type: 'priority_action', action_ref: action.action_ref });
        }
        if (state.phase !== 'first main phase') throw new Error('Fixture did not reach main phase');
        for (const subject of subjects) {
          const session = subject.seat === 1 ? owner : peer;
          const metadata = await session.call('getHiddenCardState');
          const object = metadata.objects.find(item => item.hiddenCard?.owner === subject.seat && item.hiddenCard.slot === subject.position);
          if (!object) throw new Error('Missing committed cross-owner subject');
          subject.stableId = object.stableId;
          subject.id = object.id;
          if (subject.seat === 0 && isGonti) continue;
          const secret = manifests[subject.seat].slotSecrets.find(entry => entry.slot === subject.slot);
          const reveal = { owner: subject.seat, objectId: object.id, position: subject.position,
            originalSlot: subject.slot, cardName: secret.card, commitment: secret.commitment,
            positionCommitment: `ziffle:${ceremonies[subject.seat].deckHash}:${subject.position}` };
          if (isTheft && subject.seat === 0) await setupCall('revealHiddenPosition', reveal);
          else await session.call('revealHiddenPosition', reveal);
          await setupCall('drawCard', subject.seat);
        }
        if (isTheft) {
          for (let index = 0; index < 2; index++) await setupCall('addCardToZone', 0, 'Forest', 'battlefield', true);
          const target = (await readFixture(owner)).objects.find(object => object.stableId === subjects[1].stableId);
          await castAndResolveFixtureSpell(setupCall, { objectId: target.id });
          await advanceFixtureToMain(setupCall, 1);
        }
        for (let index = 0; index < (isTheft ? 4 : isGonti ? 6 : 1); index++) {
          await setupCall('addCardToZone', 1, isTheft ? 'Island' : 'Swamp', 'battlefield', true);
        }
        const fixture = await readFixture(owner);
        subjects.forEach(subject => { subject.id = fixture.objects.find(object => object.stableId === subject.stableId).id; });
        const actorCheckpoint = fixture;
        state = await owner.call('uiState');
        const sessions = [peer, owner];
        const wrap = session => new Proxy({}, { get: (_, method) => method.startsWith('ziffle')
          ? async input => api[method](input) : (...args) => session.call(method, ...args) });
        const refs = { gameRef: { current: wrap(owner) }, stateRef: { current: state },
          multiplayerRef: { current: { localPlayerIndex: 1, players: manifests.map((manifest, index) => ({ index, deckAuditManifest: publicDeckManifest(manifest) })) } },
          verifiedAuditOpeningsRef: { current: new Set() },
          matchStartPayloadRef: { current: { auditMatchId: matchId, deckAuditManifests: manifests.map(publicDeckManifest), players: [{ index: 0 }, { index: 1 }] } },
          privateDeckManifestsRef: { current: new Map([[`${matchId}:1`, manifests[1]]]) },
          liveZiffleCeremoniesRef: { current: new Map(ceremonies.map(ceremony => [ceremony.owner, ceremony])) },
          localZiffleCeremonyLookupRef: { current: new Map(ceremonies.map(ceremony => [`initial:${ceremony.owner}`, ceremony])) },
          setState() {}, setStatus() {}, setMultiplayer() {} };
        const base = new Proxy(refs, { get: (object, key) => object[key] ||= { current: new Map() } });
        const services = { current: { collectZiffleRevealTokens: async (selected, position) => selected.tokens.filter(token => token.cardPosition === position),
          collectZiffleRevealTokensBatch: async (ceremony, selected) => ceremony.tokens.filter(token => selected.includes(token.cardPosition)),
          commandObjectHiddenRefs: command => command?.object_hidden_refs || [], commandObjectStableIds: command => command?.object_stable_ids || [],
          currentObjectIdForHiddenRef: async () => null, currentObjectIdForStableId: async () => null } };
        reactRoot = mountOpeningServices(base, services);
        const onSeat = async (seat, work) => {
          refs.gameRef.current = wrap(sessions[seat]);
          refs.multiplayerRef.current.localPlayerIndex = seat;
          refs.privateDeckManifestsRef.current = new Map([[`${matchId}:${seat}`, manifests[seat]]]);
          refs.stateRef.current = await sessions[seat].call('uiState');
          try { return await work(); }
          finally {
            refs.gameRef.current = wrap(owner); refs.multiplayerRef.current.localPlayerIndex = 1;
            refs.privateDeckManifestsRef.current = new Map([[`${matchId}:1`, manifests[1]]]); refs.stateRef.current = state;
          }
        };
        const buildOpenings = async (command, requirements) => {
          const openings = [];
          for (const seat of [0, 1]) await onSeat(seat, async () => {
            if (command) openings.push(...await services.current.buildLocalOpeningsForCommand(command, requirements));
            openings.push(...await services.current.buildLocalRequirementOpeningsForRequirements(requirements, { forceZiffleOpeningProof: true }));
          });
          return [...new Map(openings.map(opening => [`${opening.owner}:${opening.slot}:${opening.commitment}:${opening.timing || 'pre'}`, opening])).values()];
        };
        const applyBoth = async (openings, timing, command = null) => {
          for (const seat of [0, 1]) await onSeat(seat, () => services.current.revealAuditOpenings(openings,
            { timing, command, updateState: false, previewInspector: false }));
        };
        // Private transport is simulated with real Ziffle tokens and the native
        // reveal API. This does not exercise the network privacy coordinator.
        const hydratePrivate = async requirements => {
          for (const requirement of requirements.filter(item => item.type === 'private_open')) {
            const viewer = requirement.viewer;
            const session = sessions[viewer];
            if (!session) throw new Error(`Unexpected private viewer ${viewer}`);
            const checkpoint = await readFixture(session);
            const known = checkpoint.objects.find(object => object.id === requirement.objectId)
              || checkpoint.objects.find(object => object.owner === requirement.owner && object.hiddenCard?.commitment === requirement.commitment);
            if (known && known.name !== 'Hidden Card') continue;
            const position = requirement.publicSlot ?? requirement.slot;
            const ceremony = ceremonies[requirement.owner];
            const reveal = api.ziffleRevealCard({ ...ceremony, cardPosition: position,
              tokens: ceremony.tokens.filter(token => token.cardPosition === position) });
            const secret = manifests[requirement.owner].slotSecrets.find(entry => entry.slot === reveal.originalSlot);
            await session.call('revealHiddenPosition', { owner: requirement.owner, position, originalSlot: reveal.originalSlot,
              cardName: secret.card, commitment: secret.commitment,
              positionCommitment: requirement.publicCommitment ?? requirement.commitment, recomputeDecision: true });
          }
          state = await owner.call('uiState'); refs.stateRef.current = state;
        };
        const dispatchBoth = async command => {
          const entry = { command, stage: 'preview' };
          trace.push(entry);
          stage = `${trace.length}:${command.type}:preview`;
          const requirements = await owner.call('previewCryptoRequirements', command);
          entry.requirements = requirements;
          stage = `${trace.length}:${command.type}:private-hydration`;
          await hydratePrivate(requirements);
          stage = `${trace.length}:${command.type}:opening-build`;
          const prepared = await buildOpenings(command, requirements);
          entry.openings = prepared.map(opening => ({ owner: opening.owner, card: opening.card, slot: opening.slot, origin: opening.originPosition, timing: opening.timing, proof: Boolean(opening.ziffleReveal) }));
          stage = `${trace.length}:${command.type}:pre-opening`;
          await applyBoth(prepared, 'pre', command);
          stage = `${trace.length}:${command.type}:dispatch`;
          state = await owner.call('dispatch', command); refs.stateRef.current = state;
          const peerState = await peer.call('dispatch', command);
          entry.decision = state.decision;
          entry.peerDecision = peerState.decision;
          stage = `${trace.length}:${command.type}:post-opening`;
          await applyBoth(prepared, 'post');
          const afterRequirements = state.crypto_requirements || [];
          const afterOpenings = await buildOpenings(null, [...requirements, ...afterRequirements]);
          await applyBoth(afterOpenings, 'post');
          await hydratePrivate(afterRequirements);
          // Compare/inspect decisions only after all required material is applied:
          // immediately after dispatch, different private knowledge is expected.
          entry.hydratedDecision = state.decision;
          entry.hydratedPeerDecision = (await peer.call('uiState')).decision;
          const ownAudit = await owner.call('exportPublicAuditCheckpoint');
          const peerAudit = await peer.call('exportPublicAuditCheckpoint');
          entry.hashesEqual = await publicCheckpointHash(ownAudit) === await publicCheckpointHash(peerAudit);
          entry.stage = 'complete';
          if (!entry.hashesEqual) {
            entry.ownerAudit = ownAudit; entry.peerAudit = peerAudit;
            throw new Error('Public checkpoint hashes differ');
          }
        };
        const spell = fixture.objects.find(object => object.id === subjects[0].id);
        const creature = fixture.objects.find(object => object.id === subjects[1].id);
        stage = 'find-cast-action';
        const action = state.decision.actions.find(item => item.action_ref?.kind === 'cast_spell' && Number(item.object_id) === spell.id);
        if (!action) throw new Error(`Card not playable: ${JSON.stringify(state.decision)}`);
        await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: spell.id });
        for (let index = 0; index < 20; index++) {
          const checkpoint = await readFixture(owner);
          const currentSpell = checkpoint.objects.find(object => object.stableId === spell.stableId);
          if (state.decision?.kind === 'priority' && currentSpell && !['hand', 'stack'].includes(currentSpell.zone) && (!isGonti || !checkpoint.stack.length)) break;
          const decision = state.decision;
          if (decision.kind === 'targets') {
            await dispatchBoth({ type: 'select_targets', targets: [isTheft ? { kind: 'object', object: creature.id } : { kind: 'player', player: 0 }] });
          } else if (decision.kind === 'mana_payment') {
            await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
          } else if (decision.kind === 'priority') {
            const pass = decision.actions.find(item => item.action_ref?.kind === 'pass_priority');
            await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
          } else if (decision.kind === 'select_objects') {
            const chosen = decision.candidates.find(candidate => candidate.legal && candidate.name === 'Lightning Bolt')
              || decision.candidates.find(candidate => candidate.legal);
            if (!chosen) throw new Error(`No legal discard: ${JSON.stringify(decision)}`);
            const hidden = chosen.hidden_ref;
            await dispatchBoth({ type: 'select_objects', object_ids: [chosen.id], object_hidden_refs: hidden ? [{
              owner: hidden.owner, zone: hidden.zone, public_slot: hidden.public_slot, public_commitment: hidden.public_commitment,
            }] : [] });
          } else if (decision.kind === 'select_options') {
            await dispatchBoth({ type: 'select_options', option_indices: decision.options.filter(option => option.legal).slice(0, decision.min).map(option => option.index) });
          } else throw new Error(`Unexpected decision: ${JSON.stringify(decision)}`);
        }
        let exileBeforeCast = null;
        if (isGonti) {
          stage = 'opponent-exile-cast';
          const ownBeforeCast = await readFixture(owner);
          const peerBeforeCast = await readFixture(peer);
          exileBeforeCast = { actor: ownBeforeCast.objects.filter(object => object.owner === 0 && object.zone === 'exile'),
            peer: peerBeforeCast.objects.filter(object => object.owner === 0 && object.zone === 'exile') };
          const stolen = state.decision.actions.find(item => item.action_ref?.kind === 'cast_spell' && item.action_ref.from_zone === 'exile');
          if (!stolen) throw new Error(`No stolen-card cast: ${JSON.stringify(state.decision)}`);
          await dispatchBoth({ type: 'priority_action', action_ref: stolen.action_ref, object_id: stolen.object_id });
          for (let index = 0; index < 10; index++) {
            const checkpoint = await readFixture(owner);
            if (state.decision.kind === 'priority' && !checkpoint.stack.length) break;
            const decision = state.decision;
            if (decision.kind === 'targets') await dispatchBoth({ type: 'select_targets', targets: [{ kind: 'player', player: 0 }] });
            else if (decision.kind === 'mana_payment') await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
            else if (decision.kind === 'priority') await dispatchBoth({ type: 'priority_action', action_ref: decision.actions.find(item => item.action_ref?.kind === 'pass_priority').action_ref });
            else throw new Error(`Unexpected stolen-card decision: ${JSON.stringify(decision)}`);
          }
        }
        stage = 'final-checkpoint';
        const after = await readFixture(owner);
        const peerAfter = await readFixture(peer);
        let repeatedOpening = null;
        if (isTheft) {
          const controlled = after.objects.find(object => object.stableId === creature.stableId);
          const hidden = controlled.hiddenCard;
          repeatedOpening = await buildOpenings(null, [{ type: 'public_open', owner: 0, objectId: controlled.id,
            slot: hidden.slot, commitment: hidden.commitment, publicSlot: hidden.publicSlot,
            publicCommitment: hidden.publicCommitment, originSlot: hidden.originSlot, originCommitment: hidden.originCommitment, card: controlled.originalCardName }]);
          stage = 'reopen-controlled-opponent-card';
          await applyBoth(repeatedOpening, 'post');
        }
        return { cardName, stage, error: null, trace, exileBeforeCast, ceremonyDeckHashes: ceremonies.map(ceremony => ceremony.deckHash), initialActorHandKnowledge: actorCheckpoint.objects.filter(object => object.owner === 0 && object.zone === 'hand').map(object => object.name),
          opponentHandKnownByActor: after.objects.filter(object => object.owner === 0 && object.zone === 'hand').map(object => object.name),
          opponentGraveyard: after.objects.filter(object => object.owner === 0 && object.zone === 'graveyard').map(object => object.name),
          spell: after.objects.find(object => object.stableId === spell.stableId),
          creature: after.objects.find(object => object.stableId === creature.stableId),
          peerCreature: peerAfter.objects.find(object => object.stableId === creature.stableId),
          repeatedOpening: repeatedOpening?.map(opening => ({ owner: opening.owner, card: opening.card, slot: opening.slot })),
          finalHashesEqual: await publicCheckpointHash(await owner.call('exportPublicAuditCheckpoint')) === await publicCheckpointHash(await peer.call('exportPublicAuditCheckpoint')),
          life: state.players.map(player => player.life) };
      } catch (error) {
        return { cardName, stage, error: error.message, trace,
          ownerCheckpoint: await readFixture(owner), peerCheckpoint: peer ? await readFixture(peer) : null };
      } finally { reactRoot?.unmount(); owner.worker.terminate(); peer?.worker.terminate(); }
    }, { cardName, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
    assert.deepEqual(pageErrors, []);
    const summarizeObject = object => object && ({ id: object.id, stableId: object.stableId, owner: object.owner, controller: object.controller,
      zone: object.zone, name: object.name, originalCardName: object.originalCardName, faceDown: object.faceDown, hiddenCard: object.hiddenCard });
    const summarizeDecision = decision => decision && ({ kind: decision.kind, player: decision.player,
      candidates: decision.candidates?.map(candidate => ({ id: candidate.id, name: candidate.name, legal: candidate.legal })) });
    console.log(JSON.stringify({ type: 'cross-owner-edge-evidence', cardName, stage: result.stage, error: result.error,
      trace: result.trace.map(entry => ({ command: entry.command, stage: entry.stage,
        requirements: entry.requirements?.map(({ type, owner, viewer, objectId, slot, publicSlot, card, reason }) => ({ type, owner, viewer, objectId, slot, publicSlot, card, reason })),
        openings: entry.openings, decision: summarizeDecision(entry.hydratedDecision), peerDecision: summarizeDecision(entry.hydratedPeerDecision), hashesEqual: entry.hashesEqual })),
      initialActorHandKnowledge: result.initialActorHandKnowledge, opponentHandKnownByActor: result.opponentHandKnownByActor,
      ceremonyDeckHashes: result.ceremonyDeckHashes, opponentGraveyard: result.opponentGraveyard, life: result.life, finalHashesEqual: result.finalHashesEqual,
      spell: summarizeObject(result.spell), creature: summarizeObject(result.creature), peerCreature: summarizeObject(result.peerCreature),
      repeatedOpening: result.repeatedOpening, exileBeforeCast: result.exileBeforeCast && { actor: result.exileBeforeCast.actor.map(summarizeObject), peer: result.exileBeforeCast.peer.map(summarizeObject) } }));
    if (result.error) console.log(JSON.stringify(result));
    assert.equal(result.error, null, `${cardName} failed at ${result.stage}`);
    assert.notEqual(result.ceremonyDeckHashes[0], result.ceremonyDeckHashes[1]);
    assert.equal(result.finalHashesEqual, true);
    assert.ok(result.trace.every(entry => entry.hashesEqual));
    if (cardName === 'Control Magic') {
      assert.equal(result.creature.owner, 0);
      assert.equal(result.creature.controller, 1);
      assert.equal(result.peerCreature.owner, 0);
      assert.equal(result.peerCreature.controller, 1);
      assert.equal(result.creature.name, 'Grizzly Bears');
      assert.equal(result.peerCreature.name, 'Grizzly Bears');
      assert.equal(result.repeatedOpening[0].owner, 0);
    } else if (cardName === 'Gonti, Lord of Luxury') {
      assert.equal(result.exileBeforeCast.actor[0].name, 'Lightning Bolt');
      assert.equal(result.exileBeforeCast.peer[0].name, 'Hidden Card');
      assert.equal(result.exileBeforeCast.actor[0].owner, 0);
      assert.equal(result.exileBeforeCast.actor[0].faceDown, true);
      assert.equal(result.exileBeforeCast.peer[0].faceDown, true);
      const privateChoice = result.trace.find(entry => entry.hydratedDecision?.kind === 'select_objects');
      assert.deepEqual(privateChoice.hydratedDecision.candidates.map(candidate => candidate.name).sort(), ['Grizzly Bears', 'Island', 'Lightning Bolt', 'Mountain']);
      assert.ok(privateChoice.hydratedPeerDecision.candidates.every(candidate => candidate.name.toLowerCase() === 'hidden card'));
      const exileCast = result.trace.find(entry => entry.command.action_ref?.kind === 'cast_spell' && entry.command.action_ref.from_zone === 'exile');
      assert.ok(exileCast.openings.length > 0);
      assert.ok(exileCast.openings.every(opening => opening.owner === 0 && opening.card === 'Lightning Bolt' && opening.proof));
      // Gonti also requests fairness and library resealing. This probe does
      // not execute those coordinators; it covers the cross-owner proof path.
      assert.deepEqual(result.opponentGraveyard, ['Lightning Bolt']);
      assert.equal(result.life[0], 17);
    } else {
      const choices = result.trace.filter(entry => entry.hydratedDecision?.kind === 'select_objects');
      assert.equal(choices.length, 2);
      for (const entry of choices) {
        const legal = decision => decision.candidates.filter(candidate => candidate.legal).map(candidate => candidate.name).sort();
        assert.deepEqual(legal(entry.hydratedDecision), legal(entry.hydratedPeerDecision));
        assert.ok(!legal(entry.hydratedDecision).includes('Island'));
        assert.ok(!legal(entry.hydratedDecision).includes('Hidden Card'));
      }
      assert.deepEqual(choices[0].hydratedDecision.candidates.filter(candidate => candidate.legal).map(candidate => candidate.name).sort(),
        cardName === 'Thoughtseize' ? ['Grizzly Bears', 'Lightning Bolt'] : ['Lightning Bolt']);
      assert.ok(result.initialActorHandKnowledge.every(name => name === 'Hidden Card'));
      assert.deepEqual(result.opponentGraveyard, ['Lightning Bolt']);
      assert.equal(result.life[1], cardName === 'Thoughtseize' ? 18 : 20);
    }
  });
}
