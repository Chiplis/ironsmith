import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

for (const cardName of ['Saw It Coming', 'Behold the Multiverse']) {
  test(`${cardName} foretell keeps its identity private across two committed peers`, { timeout: 120000 }, async t => {
    const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
      server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await server.listen(); t.after(() => server.close());
    const browser = await chromium.launch(); t.after(() => browser.close());
    const page = await browser.newPage();
    const pageErrors = [];
    page.on('pageerror', error => pageErrors.push(error.message));
    await page.route('**/foretell-edge-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root"></div>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/foretell-edge-test`);
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
        const matchId = `foretell-edge:${cardName}`;
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
        const topSlot = reveals.find(reveal => reveal.cardPosition === deckCount - 1).originalSlot;
        const decks = [deck.slice(), deck.slice()];
        decks[0][topSlot] = 'Grizzly Bears';
        decks[1][topSlot] = cardName;
        const manifests = await Promise.all(decks.map((cards, seat) => buildPrivateDeckManifest({ matchId, owner: seat, deck: cards })));
        const ceremony = { owner: 1, deckCount, context, keys, steps, deckHash: verified.deckHash, tokens, reveals };
        peer = createWorkerSession(); await peer.ready;
        await peer.call('setPerspective', 0);
        const setupCall = async (method, ...args) => {
          const result = await owner.call(method, ...args);
          await peer.call(method, ...args);
          return result;
        };
        await owner.call('setPerspective', 1);
        let state = await setupCall('startMatch', { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1,
          format: 'normal', startingPlayer: 1, openingHandSize: 0, decks: [[], []], publicDecklists: decks,
          hiddenDeckManifests: manifests.map((manifest, seat) => buildZiffleRuntimeManifest(manifest, { ...ceremony, owner: seat })) });
        for (let index = 0; index < 30 && state.phase !== 'first main phase'; index++) {
          const action = state.decision?.actions?.find(item => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(item.action_ref?.kind));
          if (!action) throw new Error(`Unexpected pregame decision ${JSON.stringify(state.decision)}`);
          state = await setupCall('dispatch', { type: 'priority_action', action_ref: action.action_ref });
        }
        if (state.phase !== 'first main phase') throw new Error('Fixture did not reach main phase');
        const subjects = [{ seat: 1, slot: topSlot }, { seat: 0, slot: topSlot }];
        for (const subject of subjects) {
          const session = subject.seat === 1 ? owner : peer;
          const metadata = await session.call('getHiddenCardState');
          const position = deckCount - 1;
          const object = metadata.objects.find(item => item.hiddenCard?.owner === subject.seat && item.hiddenCard.slot === position);
          if (!object) throw new Error('Missing committed fixture subject');
          const secret = manifests[subject.seat].slotSecrets.find(entry => entry.slot === subject.slot);
          // The opponent creature is a known fixture target; the foretold
          // card is opened only on its owner's engine.
          const revealCall = subject.seat === 0 ? setupCall : session.call;
          await revealCall('revealHiddenPosition', { owner: subject.seat, objectId: object.id, position,
            originalSlot: subject.slot, cardName: secret.card, commitment: secret.commitment,
            positionCommitment: `ziffle:${ceremony.deckHash}:${position}` });
          await setupCall('drawCard', subject.seat);
          const drawn = await session.call('getHiddenCardState');
          subject.id = drawn.objects.find(item => item.stableId === object.stableId).id;
        }
        for (let index = 0; index < 8; index++) await setupCall('addCardToZone', 1, 'Island', 'battlefield', true);
        for (let index = 0; index < 2; index++) await setupCall('addCardToZone', 0, 'Forest', 'battlefield', true);
        state = await owner.call('uiState');
        const fixture = await owner.call('getHiddenCardState');
        const peerBefore = await peer.call('getHiddenCardState');
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
          if (state.decision?.kind !== peerState.decision?.kind) throw new Error(`Peer decisions differ: owner=${JSON.stringify(state.decision)} peer=${JSON.stringify(peerState.decision)}`);
          stage = `${trace.length}:${command.type}:post-opening`;
          await applyPeer(prepared, 'post');
          const afterRequirements = state.crypto_requirements || state.cryptoRequirements || [];
          const afterOpenings = await buildOpenings(null, [...requirements, ...afterRequirements]);
          await applyPeer(afterOpenings, 'post');
          stage = `${trace.length}:${command.type}:public-hash`;
          const ownHash = await publicCheckpointHash(await owner.call('exportPublicAuditCheckpoint'));
          const peerHash = await publicCheckpointHash(await peer.call('exportPublicAuditCheckpoint'));
          if (ownHash !== peerHash) throw new Error(`Public checkpoint hashes differ: ${ownHash} / ${peerHash}`);
          trace.push({ command, decision: state.decision?.kind, prepared: prepared.map(opening => ({ owner: opening.owner, slot: opening.slot, origin: opening.originPosition, card: opening.card })),
            requirements: [...requirements, ...afterRequirements].map(requirement => ({ type: requirement.type, owner: requirement.owner, slot: requirement.slot, from: requirement.from, to: requirement.to })) });
        };
        const spell = fixture.objects.find(object => object.id === subjects[0].id);
        const creature = fixture.objects.find(object => object.id === subjects[1].id);
        stage = 'find-cast-action';
        const action = state.decision?.actions?.find(item => item.action_ref?.kind === 'special_action' && item.action_ref.action?.kind === 'foretell' && Number(item.object_id) === spell.id);
        if (!action) throw new Error(`Card not playable: ${JSON.stringify(state.decision)}`);
        const peerSavepoint = await peer.call('createRuntimeSavepoint');
        let unhydratedReplayError = null;
        try { await peer.call('dispatch', { type: 'priority_action', action_ref: action.action_ref, object_id: spell.id }); }
        catch (error) { unhydratedReplayError = error.message; }
        await peer.call('restoreRuntimeSavepoint', peerSavepoint);
        await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: spell.id });
        for (let index = 0; index < 4 && state.decision?.kind !== 'priority'; index++) {
          const decision = state.decision;
          if (decision.kind !== 'mana_payment') throw new Error(`Unexpected foretell decision ${JSON.stringify(decision)}`);
          await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
        }
        const after = await owner.call('getHiddenCardState');
        const peerAfter = await peer.call('getHiddenCardState');
        const privateCommandCount = trace.length;
        const foretold = after.objects.find(object => object.stableId === spell.stableId);
        foretold.faceDown = state.players.flatMap(player => player.exile_cards).find(card => card.id === foretold.id)?.face_down;
        if (state.decision.actions.some(item => item.action_ref?.kind === 'cast_spell' && Number(item.object_id) === foretold.id)) {
          throw new Error('Foretold card was castable on the turn it was foretold');
        }
        // Advance real priority commands to the next player's main phase.
        // The foretell card stays private throughout the turn boundary.
        const foretellTurn = state.turn_number;
        for (let index = 0; index < 40 && !(state.turn_number > foretellTurn && state.phase === 'first main phase'); index++) {
          if (state.decision.kind === 'attackers' && state.decision.attacker_options.length === 0) {
            await dispatchBoth({ type: 'declare_attackers', declarations: [], bands: [] });
          } else if (state.decision.kind === 'blockers') {
            await dispatchBoth({ type: 'declare_blockers', declarations: [] });
          } else {
            const pass = state.decision?.actions?.find(item => item.action_ref?.kind === 'pass_priority');
            if (!pass) throw new Error(`Unexpected turn-advance decision ${JSON.stringify(state.decision)}`);
            await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
          }
        }
        if (state.turn_number <= foretellTurn) throw new Error('Fixture did not reach a later turn');
        if (cardName === 'Saw It Coming') {
          const opponentState = await peer.call('uiState');
          const targetSpell = opponentState.decision?.actions?.find(item => item.action_ref?.kind === 'cast_spell' && Number(item.object_id) === creature.id);
          if (!targetSpell) throw new Error('Missing opponent creature cast for counterspell target');
          await dispatchBoth({ type: 'priority_action', action_ref: targetSpell.action_ref, object_id: creature.id });
          if (state.decision.kind !== 'mana_payment') throw new Error('Opponent creature did not request payment');
          await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash } });
        }
        if (state.decision.player !== 1) {
          const pass = state.decision.actions.find(item => item.action_ref?.kind === 'pass_priority');
          await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
        }
        stage = 'cast-foretold-card';
        const laterCast = state.decision.actions.find(item => item.action_ref?.kind === 'cast_spell'
          && item.action_ref.from_zone === 'exile' && Number(item.object_id) === foretold.id);
        if (!laterCast) throw new Error(`Missing foretold cast on later turn: ${JSON.stringify(state.decision)}`);
        const castCommandIndex = trace.length;
        await dispatchBoth({ type: 'priority_action', action_ref: laterCast.action_ref, object_id: foretold.id });
        if (state.decision.kind === 'targets') {
          await dispatchBoth({ type: 'select_targets', targets: [state.decision.requirements[0].legal_targets[0]] });
        }
        if (state.decision.kind !== 'mana_payment') throw new Error('Foretold cast did not request payment');
        await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash } });
        const castCheckpoint = await owner.call('getHiddenCardState');
        const castPeerCheckpoint = await peer.call('getHiddenCardState');
        return { cardName, trace, unhydratedReplayError, privateCommandCount, castCommandIndex,
          laterSpell: castCheckpoint.objects.find(object => object.stableId === spell.stableId),
          laterPeerSpell: castPeerCheckpoint.objects.find(object => object.stableId === spell.stableId),
          peerBeforeSpell: peerBefore.objects.find(object => object.stableId === spell.stableId),
          afterSpell: after.objects.find(object => object.stableId === spell.stableId),
          peerSpell: peerAfter.objects.find(object => object.stableId === spell.stableId),
          decision: state.decision?.kind };
      } catch (error) {
        throw new Error(`${cardName} failed at ${stage}: ${error.message}; trace=${JSON.stringify(trace)}`);
      } finally { reactRoot?.unmount(); owner.worker.terminate(); peer?.worker.terminate(); }
    }, { cardName, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
    assert.deepEqual(pageErrors, []);
    assert.equal(result.peerBeforeSpell.name, 'Hidden Card');
    assert.equal(result.unhydratedReplayError, null, 'a committed placeholder must replay foretell without an opening');
    assert.deepEqual(result.trace.slice(0, result.privateCommandCount).flatMap(step => step.prepared), [], 'foretell itself must produce no public openings');
    assert.equal(result.decision, 'priority');
    const reportDir = path.resolve(root, '../../reports/player-mcp/zkp-protocol-fixes-2026-09-27');
    await mkdir(reportDir, { recursive: true });
    await writeFile(path.join(reportDir, `foretell-${cardName.toLowerCase().replaceAll(' ', '-')}-evidence.json`), JSON.stringify(result, null, 2) + '\n');
    t.diagnostic(JSON.stringify({ cardName, commands: result.trace.length, unhydratedReplayError: result.unhydratedReplayError,
      publicOpenings: result.trace.flatMap(step => step.prepared), opponentName: result.peerSpell.name, opponentOriginalName: result.peerSpell.originalCardName }));
    assert.equal(result.afterSpell.zone, 'exile');
    assert.equal(result.afterSpell.faceDown, true);
    assert.equal(result.peerSpell.originalCardName ?? null, null, 'foretell must not open the printed identity to the opponent');
    assert.equal(result.peerSpell.name, 'Hidden Card');
    assert.equal(result.laterSpell.zone, 'stack');
    assert.equal(result.laterPeerSpell.zone, 'stack');
    assert.equal(result.laterPeerSpell.originalCardName, cardName);
    assert.ok(result.trace[result.castCommandIndex].prepared.some(opening => opening.card === cardName), 'the later cast must carry the real identity proof');
  });
}
