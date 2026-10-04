import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

// Controlled board setup; every shuffle, reveal and subsequent game decision
// below uses the real verifier WASM and two independently executing engines.
test('private ciphertext epochs survive repeated fetches and a returned card without retaining old identities', { timeout: 240000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  const pageErrors = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  await page.route('**/private-shuffle-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Private shuffle regression</title>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/private-shuffle-test`);
  const result = await page.evaluate(async verifierUrl => {
    const api = await import(verifierUrl); await api.default();
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const { buildPrivateDeckManifest, publicCheckpointHash } = await import('/src/lib/multiplayer-audit.js');
    const { buildZiffleRuntimeManifest } = await import('/src/lib/ziffle-runtime-manifest.js');
    const { buildZiffleInputDeck, assertZiffleEpochInputs, assertZiffleEpochVerification, ziffleEpochMaterial } = await import('/src/lib/ziffle-private-epochs.js');
    function workerSession() {
      const decoder = createSnapshotDecoder(), pending = new Map(), analyses = new Map(), waiters = new Map();
      const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
      let id = 0;
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
        pending.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
      }).then(async value => {
        if (value?.decision?.analysis_complete !== false) return value;
        const revision = value.__priority_revision;
        const decision = analyses.get(revision) || await new Promise(resolve => waiters.set(revision, resolve));
        waiters.delete(revision); return { ...value, decision };
      });
      worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
      return { call, ready, stop: () => worker.terminate() };
    }
    const owner = workerSession(), peer = workerSession();
    const trace = [], captures = [], accepted = [];
    let state, stage = 'setup';
    try {
      await Promise.all([owner.ready, peer.ready]);
      const matchId = 'private-shuffle-browser';
      const deck = Array.from({ length: 60 }, (_, slot) => slot === 3 ? 'Emperor of Bones' : slot % 2 ? 'Swamp' : 'Island');
      const manifests = await Promise.all([deck, Array(60).fill('Mountain')].map((cards, seat) => buildPrivateDeckManifest({ matchId, owner: seat, deck: cards })));
      const identities = ['11', '22'].map(byte => api.ziffleKeygen({ deckCount: 60, context: matchId, entropyHex: byte.repeat(32) }));
      const keys = identities.map((identity, player) => ({ player, publicKeyHex: identity.publicKeyHex, ownershipProofHex: identity.ownershipProofHex }));
      let nonce = 1;
      const finish = input => {
        const ceremony = { ...input, steps: [] };
        for (let shuffler = 0; shuffler < 2; shuffler++) {
          const step = api.ziffleBuildShuffleStep({ ...ceremony, shuffler, entropyHex: (++nonce).toString(16).padStart(64, '0') });
          ceremony.steps.push({ shuffler, deckHex: step.deckHex, proofHex: step.proofHex });
        }
        return { ...ceremony, ...api.ziffleVerifyShuffle(ceremony) };
      };
      const genesis = [0, 1].map(seat => finish({ owner: seat, deckCount: 60, context: `${matchId}:initial:${seat}`, keyContext: matchId, keys }));
      accepted.push(genesis[0]);
      const revealPositions = (ceremony, positions) => {
        const tokens = identities.flatMap(identity => api.ziffleBuildRevealTokens({ ...ceremony, ...identity,
          deckCount: ceremony.deckCount, cardPositions: positions, entropyHex: (++nonce).toString(16).padStart(64, '0') }));
        return api.ziffleRevealCards({ ...ceremony, cardPositions: positions, tokens });
      };
      const opening = (requirement, checkpoint) => {
        let object = checkpoint.objects.find(item => item.id === Number(requirement.objectId));
        const ref = requirement.publicCommitment || object?.hiddenCard?.publicCommitment || requirement.commitment;
        object ||= checkpoint.objects.find(item => Number(item.hiddenCard?.owner) === Number(requirement.owner)
          && [item.hiddenCard?.publicCommitment, item.hiddenCard?.commitment].includes(ref));
        if (!object) throw new Error(`Missing current ciphertext object for ${JSON.stringify(requirement)}`);
        const match = /^ziffle:([^:]+):(\d+)$/.exec(ref || '');
        if (!match) throw new Error(`No ciphertext provenance: ${JSON.stringify(requirement)}`);
        const ceremony = accepted.find(entry => entry.deckHash === match[1]);
        if (!ceremony) throw new Error(`Unknown ciphertext epoch ${ref}`);
        const position = Number(match[2]);
        const revealed = revealPositions(ceremony, [position])[0];
        const secret = manifests[0].slotSecrets.find(entry => entry.slot === revealed.originalSlot);
        return { owner: 0, objectId: object.id, position, originalSlot: secret.slot, cardName: secret.card,
          positionCommitment: ref, commitment: secret.commitment };
      };
      // Reproduce setup commands on each engine, with each client's own view.
      const setupCall = async (method, ...args) => {
        const result = await owner.call(method, ...args);
        await peer.call(method, ...args);
        return result;
      };
      await owner.call('setPerspective', 0);
      await peer.call('setPerspective', 1);
      state = await setupCall('startMatch', { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1, format: 'normal',
        startingPlayer: 0, openingHandSize: 0, decks: [[], []], publicDecklists: [deck, Array(60).fill('Mountain')],
        hiddenDeckManifests: manifests.map((manifest, seat) => buildZiffleRuntimeManifest(manifest, genesis[seat])) });
      for (let index = 0; index < 40 && state.phase !== 'first main phase'; index++) {
        const action = state.decision?.actions?.find(item => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(item.action_ref?.kind));
        if (!action) throw new Error(`Cannot reach fixture main phase: ${JSON.stringify(state.decision)}`);
        state = await setupCall('dispatch', { type: 'priority_action', action_ref: action.action_ref });
      }
      if (state.phase !== 'first main phase') throw new Error('Fixture did not reach main phase');
      const initialLibraryCount = (await owner.call('getHiddenCardState')).objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library').length;
      const fetches = [];
      for (const name of ['Marsh Flats', 'Polluted Delta', 'Polluted Delta']) fetches.push(await setupCall('addCardToZone', 0, name, 'battlefield', true));
      const returnSpellId = await setupCall('addCardToZone', 0, 'Temporal Eddy', 'hand', true);
      for (let i = 0; i < 4; i++) await setupCall('addCardToZone', 0, 'Island', 'battlefield', true);
      state = await owner.call('uiState');
      const dispatchBoth = async command => {
        stage = `${trace.length}:${command.type}`;
        let requirements = await owner.call('previewCryptoRequirements', command);
        const before = await owner.call('getHiddenCardState');
        const privateRequirements = requirements.filter(req => req.type === 'private_open' && Number(req.owner) === 0);
        if (privateRequirements.length) {
          await owner.call('revealHiddenPositions', { reveals: privateRequirements.map(req => opening(req, before)), recomputeDecision: true });
          requirements = await owner.call('previewCryptoRequirements', command);
        }
        const publicRequirements = requirements.filter(req => req.type === 'public_open' && Number(req.owner) === 0 && req.objectId != null);
        if (publicRequirements.length) {
          const checkpoint = await owner.call('getHiddenCardState');
          const reveals = publicRequirements.map(req => opening(req, checkpoint));
          await owner.call('revealHiddenPositions', { reveals, recomputeDecision: true });
          await peer.call('revealHiddenPositions', { reveals, recomputeDecision: true });
          requirements = await owner.call('previewCryptoRequirements', command);
        }
        const shuffles = requirements.filter(req => req.type === 'verifiable_shuffle');
        const epochs = [];
        for (const requirement of shuffles) {
          const inputs = requirement.inputCommitments ?? requirement.input_commitments;
          if (!inputs?.length) throw new Error(`Missing exact shuffle input commitments: ${JSON.stringify(requirement)}`);
          const ceremony = finish({ owner: 0, deckCount: inputs.length, context: `${matchId}:epoch:${accepted.length}`,
            keyContext: matchId, keys, inputDeck: buildZiffleInputDeck(accepted, inputs) });
          const expectedInputs = assertZiffleEpochInputs(ceremony, requirement, accepted);
          assertZiffleEpochVerification(ceremony, api.ziffleVerifyShuffle(ceremony), accepted);
          const material = ziffleEpochMaterial(ceremony, requirement, expectedInputs);
          await owner.call('queueVerifiedHiddenLibraryEpoch', material);
          await peer.call('queueVerifiedHiddenLibraryEpoch', material);
          accepted.push(ceremony); epochs.push(ceremony);
        }
        state = await owner.call('dispatch', command);
        const other = await peer.call('dispatch', command);
        if (state.decision?.kind !== other.decision?.kind) throw new Error(`Peer decisions differ at ${stage}`);
        const ownPublic = await owner.call('exportPublicAuditCheckpoint');
        const peerPublic = await peer.call('exportPublicAuditCheckpoint');
        if (await publicCheckpointHash(ownPublic) !== await publicCheckpointHash(peerPublic)) throw new Error(`Peer public hashes differ at ${stage}`);
        for (const ceremony of epochs) {
          const after = await owner.call('getHiddenCardState');
          const oldLibrary = before.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library');
          const currentLibrary = after.objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library');
          const publicLibrary = ownPublic.objects.filter(object => object.owner === 0 && object.zone === 'library');
          captures.push({ epoch: accepted.indexOf(ceremony), count: currentLibrary.length,
            sourceEpochs: ceremony.inputDeck.sources.map(source => source.epoch),
            oldIds: oldLibrary.map(object => object.id), oldStableIds: oldLibrary.map(object => object.stableId),
            ids: currentLibrary.map(object => object.id), stableIds: currentLibrary.map(object => object.stableId),
            names: currentLibrary.map(object => object.name), publicLibrary,
            deckHash: ceremony.deckHash, inputCount: ceremony.inputDeck.sources.length });
        }
        trace.push({ command: command.type, decision: state.decision?.kind, shuffles: epochs.length });
      };
      const reachSearch = async id => {
        let activated = false;
        for (let i = 0; i < 60; i++) {
          const actions = state.decision?.actions || [];
          let action = !activated && actions.find(item => item.action_ref?.kind === 'activate_ability' && Number(item.action_ref.source) === Number(id));
          if (action) activated = true;
          action ||= actions.find(item => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(item.action_ref?.kind));
          if (!action) throw new Error(`Cannot reach fetch choice: ${JSON.stringify(state.decision)}`);
          await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref });
          if (state.decision?.kind === 'select_objects') return;
        }
        throw new Error('Search decision not reached');
      };
      const searchNames = [], peerLibraryNames = [];
      const fetch = async (id, cardName) => {
        await reachSearch(id);
        searchNames.push(state.decision.candidates.map(card => card.name));
        peerLibraryNames.push((await peer.call('getHiddenCardState')).objects.filter(object => object.hiddenCard?.owner === 0 && object.zone === 'library').map(object => object.name));
        const target = state.decision.candidates.find(card => card.name === cardName && card.legal);
        if (!target) throw new Error(`No ${cardName} fetch choice: ${JSON.stringify(state.decision)}`);
        await dispatchBoth({ type: 'select_objects', object_ids: [target.id], object_hidden_refs: [{
          owner: target.hidden_ref.owner, zone: target.hidden_ref.zone,
          public_slot: target.hidden_ref.public_slot, public_commitment: target.hidden_ref.public_commitment }] });
      };
      await fetch(fetches[0], 'Swamp');
      await fetch(fetches[1], 'Island');
      const fetchedSwamp = (await owner.call('getHiddenCardState')).objects.find(object => object.hiddenCard?.owner === 0 && object.zone === 'battlefield' && object.name === 'Swamp');
      let cast = false;
      for (let i = 0; i < 30; i++) {
        const decision = state.decision;
        if (!cast) {
          const action = decision?.actions?.find(item => item.action_ref?.kind === 'cast_spell' && Number(item.object_id) === Number(returnSpellId));
          if (action) { await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: returnSpellId }); cast = true; continue; }
        }
        if (cast && (await owner.call('getHiddenCardState')).objects.some(object => object.stableId === fetchedSwamp.stableId && object.zone === 'library')) break;
        if (decision?.kind === 'targets') await dispatchBoth({ type: 'select_targets', targets: [{ kind: 'object', object: fetchedSwamp.id }] });
        else if (decision?.kind === 'number') await dispatchBoth({ type: 'number_choice', value: 0 });
        else if (decision?.kind === 'mana_payment') await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
        else {
          const action = decision?.actions?.find(item => item.action_ref?.kind === 'pass_priority');
          if (!action) throw new Error(`Cannot resolve returned card: ${JSON.stringify(decision)}`);
          await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref });
        }
      }
      const returned = (await owner.call('getHiddenCardState')).objects.find(object => object.stableId === fetchedSwamp.stableId && object.zone === 'library');
      if (!returned) throw new Error('Temporal Eddy did not return the fetched card');
      await fetch(fetches[2], 'Island');
      return { trace, captures, searchNames, peerLibraryNames, initialLibraryCount, returnedCommitment: returned.hiddenCard?.publicCommitment };
    } catch (error) { throw new Error(`${stage}: ${error.message}; trace=${JSON.stringify(trace)}`); }
    finally { owner.stop(); peer.stop(); }
  }, `/@fs${path.resolve(root, '../wasm_demo/pkg/verifier.js')}`);
  assert.deepEqual(pageErrors, []);
  assert.equal(result.captures.length, 3, 'all three actual fetch shuffles install authenticated epochs');
  assert.deepEqual(result.captures.map(epoch => epoch.count), [result.initialLibraryCount - 1, result.initialLibraryCount - 2, result.initialLibraryCount - 2]);
  for (const capture of result.captures) {
    assert.equal(capture.count, capture.inputCount);
    assert.ok(capture.ids.every(id => !capture.oldIds.includes(id)), 'all participating object IDs retire');
    assert.ok(capture.stableIds.every(id => !capture.oldStableIds.includes(id)), 'all participating stable IDs retire');
    assert.ok(capture.names.every(name => name === 'Hidden Card'), 'private knowledge is erased after each shuffle');
    for (const object of capture.publicLibrary) {
      assert.equal(object.name, 'Hidden Card');
      const serialized = JSON.stringify(object.hiddenCard);
      assert.ok(serialized.includes(capture.deckHash), 'public origin identifies only the fresh ciphertext epoch');
      assert.ok(!serialized.includes(result.returnedCommitment), 'the returned physical card cannot be linked to a fresh position');
    }
  }
  assert.ok(result.captures[2].sourceEpochs.includes(0), 'third shuffle authenticates the returned original-epoch card');
  for (const names of result.searchNames) assert.ok(names.length && names.every(name => ['Island', 'Swamp'].includes(name)));
  for (const names of result.peerLibraryNames.slice(0, 2)) assert.ok(names.every(name => name === 'Hidden Card'), 'opponent does not learn private search identities');
  const knownBeforeThird = result.peerLibraryNames[2].filter(name => name !== 'Hidden Card');
  assert.ok(knownBeforeThird.length <= 1 && knownBeforeThird.every(name => name === 'Swamp'), 'only the publicly returned Swamp may be known before the third shuffle');
  assert.ok(result.returnedCommitment?.startsWith('ziffle:'), 'returned physical card retains authenticated provenance until its next shuffle');
});
