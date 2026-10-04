import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

for (const cardName of ['Unsummon', 'Unexpectedly Absent', 'Boomerang']) {
  test(`${cardName} preserves committed identity through a hidden-zone return and recast`, { timeout: 120000 }, async t => {
    const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
      server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await server.listen(); t.after(() => server.close());
    const browser = await chromium.launch(); t.after(() => browser.close());
    const page = await browser.newPage();
    const pageErrors = [];
    page.on('pageerror', error => pageErrors.push(error.message));
    await page.route('**/exile-reentry-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root"></div>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/exile-reentry-test`);
    const result = await page.evaluate(async ({ verifierUrl, cardName }) => {
      const { mountOpeningServices } = await import('/tests/ziffle-public-opening-zone-change-harness.js');
      const { buildPrivateDeckManifest, publicDeckManifest, publicCheckpointHash } = await import('/src/lib/multiplayer-audit.js');
      const { buildZiffleRuntimeManifest } = await import('/src/lib/ziffle-runtime-manifest.js');
      const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
      const subjectName = cardName === 'Boomerang' ? 'Sink into Stupor' : 'Clone';
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
        const matchId = `zone-origin:${cardName}`;
        const deck = Array(61).fill('Mountain');
        deck[4] = subjectName;
        deck[5] = cardName;
        deck[6] = 'Think Twice';
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
          const audit = await session.call('exportPublicAuditCheckpoint');
          return { ...metadata, objects: metadata.objects.map(object => ({ ...object,
            owner: object.hiddenCard?.owner ?? audit.objects.find(card => card.id === object.id)?.owner,
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
        const subjects = [4, 5, 6].map(slot => ({ seat: 1, slot, zone: 'hand' }));
        for (const subject of subjects) {
          const checkpoint = await readFixture(owner);
          const position = reveals.find(reveal => reveal.originalSlot === subject.slot).cardPosition;
          const object = checkpoint.objects.find(item => item.owner === subject.seat && item.hiddenCard?.slot === position);
          if (!object) throw new Error(`Missing fixture subject ${JSON.stringify(subject)}`);
          const secret = manifests[subject.seat].slotSecrets.find(entry => entry.slot === subject.slot);
          await owner.call('revealHiddenPosition', { owner: subject.seat, objectId: object.id, position,
            originalSlot: subject.slot, cardName: secret.card, commitment: secret.commitment,
            positionCommitment: `ziffle:${ceremony.deckHash}:${position}` });
          subject.stableId = object.stableId;
          subject.position = position;
        }
        const lowestPosition = Math.min(...subjects.map(subject => subject.position));
        for (let index = deckCount - 1; index >= lowestPosition; index--) await setupCall('drawCard', 1);
        for (let index = 0; index < 12; index++) await setupCall('addCardToZone', 1, 'Island', 'battlefield', true);
        for (let index = 0; index < 4; index++) await setupCall('addCardToZone', 1, 'Plains', 'battlefield', true);
        await setupCall('addCardToZone', 0, 'Grizzly Bears', 'battlefield', true);
        state = await owner.call('uiState');
        const fixture = await readFixture(owner);
        subjects.forEach(subject => { subject.id = fixture.objects.find(object => object.stableId === subject.stableId).id; });
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
        const committedClone = fixture.objects.find(object => object.id === subjects[0].id);
        const returnSpell = fixture.objects.find(object => object.id === subjects[1].id);
        const drawSpell = fixture.objects.find(object => object.id === subjects[2].id);
        const snapshots = [];
        const currentObject = async stableId => (await readFixture(owner)).objects.find(object => object.stableId === stableId);
        const castAndResolve = async (spellStableId, { targetStableId = null, copy = false } = {}) => {
          const before = await currentObject(spellStableId);
          stage = `cast ${before?.name}`;
          const action = state.decision?.actions?.find(item => item.action_ref?.kind === 'cast_spell' && Number(item.object_id) === before.id);
          if (!action) throw new Error(`No cast action for ${before?.name}: ${JSON.stringify(state.decision)}`);
          await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: before.id });
          for (let index = 0; index < 15; index++) {
            const current = await currentObject(spellStableId);
            if (state.decision?.kind === 'priority' && current && !['hand', 'stack'].includes(current.zone)) return current;
            const decision = state.decision;
            if (decision?.kind === 'targets') {
              const target = await currentObject(targetStableId);
              if (!target) throw new Error('Expected target missing');
              await dispatchBoth({ type: 'select_targets', targets: [{ kind: 'object', object: target.id }] });
            } else if (decision?.kind === 'number') {
              await dispatchBoth({ type: 'number_choice', value: 0 });
            } else if (decision?.kind === 'mana_payment') {
              await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
            } else if (decision?.kind === 'priority') {
              const pass = decision.actions.find(item => item.action_ref?.kind === 'pass_priority');
              if (!pass) throw new Error('No pass action');
              await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
            } else if (decision?.kind === 'select_options' && copy) {
              const option = decision.options.find(item => item.description.includes('Enter as a copy of Grizzly Bears'));
              if (!option) throw new Error(`No copy option: ${JSON.stringify(decision)}`);
              await dispatchBoth({ type: 'select_options', option_indices: [option.index] });
            } else throw new Error(`Unexpected resolution decision: ${JSON.stringify(decision)}`);
          }
          throw new Error('Resolution did not complete');
        };
        if (cardName === 'Boomerang') {
          const action = state.decision.actions.find(item => item.action_ref?.kind === 'play_land' && Number(item.object_id) === committedClone.id);
          if (!action) throw new Error(`No MDFC land action: ${JSON.stringify(state.decision)}`);
          await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: committedClone.id });
          if (state.decision.kind !== 'select_options') throw new Error('Missing MDFC life-payment choice');
          await dispatchBoth({ type: 'select_options', option_indices: [state.decision.options.find(option => option.description === 'No').index] });
        } else await castAndResolve(committedClone.stableId, { copy: true });
        snapshots.push({ label: 'first-public', object: await currentObject(committedClone.stableId) });
        await castAndResolve(returnSpell.stableId, { targetStableId: committedClone.stableId });
        snapshots.push({ label: 'returned', object: await currentObject(committedClone.stableId) });
        if (cardName === 'Unexpectedly Absent') {
          await castAndResolve(drawSpell.stableId);
          snapshots.push({ label: 'drawn', object: await currentObject(committedClone.stableId) });
        }
        if (cardName === 'Boomerang') {
          const bear = fixture.objects.find(object => object.name === 'Grizzly Bears');
          await castAndResolve(committedClone.stableId, { targetStableId: bear.stableId });
        } else await castAndResolve(committedClone.stableId, { copy: true });
        snapshots.push({ label: 'second-public', object: await currentObject(committedClone.stableId) });
        const after = await readFixture(owner);
        const peerAfter = await readFixture(peer);
        return { cardName, subjectName, trace, snapshots, beforeClone: committedClone,
          peerBeforeClone: peerBefore.objects.find(object => object.stableId === committedClone.stableId),
          afterClone: after.objects.find(object => object.stableId === committedClone.stableId),
          peerClone: peerAfter.objects.find(object => object.stableId === committedClone.stableId),
          decision: state.decision?.kind };
      } catch (error) {
        throw new Error(`${cardName} failed at ${stage}: ${error.message}; trace=${JSON.stringify(trace)}`);
      } finally { reactRoot?.unmount(); owner.worker.terminate(); peer?.worker.terminate(); }
    }, { cardName, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
    assert.deepEqual(pageErrors, []);
    const compactObject = object => ({ id: object.id, stableId: object.stableId, owner: object.owner,
      zone: object.zone, name: object.name, originalCardName: object.originalCardName, hiddenCard: object.hiddenCard });
    console.log(`HIDDEN_RETURN_EVIDENCE ${JSON.stringify({ cardName, subjectName: result.subjectName,
      actions: result.trace.length, publicHashesChecked: result.trace.length, trace: result.trace,
      snapshots: result.snapshots.map(snapshot => ({ label: snapshot.label, object: compactObject(snapshot.object) })),
      initialPeerName: result.peerBeforeClone.name,
      finalPeersEqual: JSON.stringify(result.peerClone) === JSON.stringify(result.afterClone),
    })}`);
    assert.equal(result.peerBeforeClone.name, 'Hidden Card');
    assert.equal(result.decision, 'priority');
    assert.equal(result.snapshots[0].object.name, cardName === 'Boomerang' ? 'Soporific Springs' : 'Grizzly Bears');
    assert.equal(result.snapshots[0].object.zone, 'battlefield');
    assert.equal(result.snapshots[1].object.name, result.subjectName);
    assert.equal(result.snapshots[1].object.originalCardName, result.subjectName);
    assert.equal(result.snapshots[1].object.zone, cardName === 'Unexpectedly Absent' ? 'library' : 'hand');
    if (cardName === 'Unexpectedly Absent') {
      assert.equal(result.snapshots[2].object.name, 'Clone');
      assert.equal(result.snapshots[2].object.zone, 'hand');
    }
    assert.equal(result.afterClone.name, cardName === 'Boomerang' ? 'Sink into Stupor' : 'Grizzly Bears');
    assert.equal(result.afterClone.originalCardName, result.subjectName);
    assert.equal(result.afterClone.zone, cardName === 'Boomerang' ? 'graveyard' : 'battlefield');
    assert.deepEqual(result.peerClone, result.afterClone);
    for (const snapshot of result.snapshots) {
      assert.equal(snapshot.object.hiddenCard.slot, 4);
      assert.equal(snapshot.object.hiddenCard.originSlot, result.beforeClone.hiddenCard.originSlot);
      assert.equal(snapshot.object.hiddenCard.originCommitment, result.beforeClone.hiddenCard.originCommitment);
    }
  });
}
