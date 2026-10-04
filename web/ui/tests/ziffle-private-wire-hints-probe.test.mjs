import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { collectZiffleRevealTokenGroups } from '../src/lib/ziffle-reveal-token-collection.js';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

// Controlled board setup; every shuffle, reveal and subsequent game decision
// below uses the real verifier WASM and two independently executing engines.
test('private scry knowledge is omitted from later fetch token requests', { timeout: 240000 }, async t => {
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
    const trace = [], captures = [], accepted = [], requirementCaptures = [];
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
      const returnSpellId = await setupCall('addCardToZone', 0, 'Serum Visions', 'hand', true);
      for (let i = 0; i < 4; i++) await setupCall('addCardToZone', 0, 'Island', 'battlefield', true);
      state = await owner.call('uiState');
      const dispatchBoth = async command => {
        stage = `${trace.length}:${command.type}`;
        let requirements = await owner.call('previewCryptoRequirements', command);
        const before = await owner.call('getHiddenCardState');
        const privateRequirements = requirements.filter(req => req.type === 'private_open' && Number(req.owner) === 0);
        if (privateRequirements.length) {
          requirementCaptures.push({ command, requirements: structuredClone(requirements), ceremony: structuredClone(accepted.at(-1)) });
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
      let cast = false, sawScry = false;
      for (let i = 0; i < 50; i++) {
        const decision = state.decision;
        if (!cast) {
          const action = decision?.actions?.find(item => item.action_ref?.kind === 'cast_spell' && Number(item.object_id ?? item.action_ref.spell_id) === Number(returnSpellId));
          if (action) { await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: returnSpellId }); cast = true; continue; }
        }
        if (cast && sawScry && decision?.kind === 'priority') break;
        if (decision?.kind === 'select_objects') { sawScry = true; await dispatchBoth({ type: 'select_objects', object_ids: [] }); }
        else if (decision?.kind === 'select_options') await dispatchBoth({ type: 'select_options', option_indices: decision.options.filter(option => option.legal).slice(0, decision.min).map(option => option.index) });
        else if (decision?.kind === 'number') await dispatchBoth({ type: 'number_choice', value: 0 });
        else if (decision?.kind === 'mana_payment') await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
        else {
          const action = decision?.actions?.find(item => item.action_ref?.kind === 'pass_priority');
          if (!action) throw new Error(`Cannot resolve Serum Visions: ${JSON.stringify(decision)}`);
          await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref });
        }
      }
      if (!sawScry) throw new Error('Serum Visions scry not reached');
      await reachSearch(fetches[1]);
      return { trace, requirementCaptures };
    } catch (error) { throw new Error(`${stage}: ${error.message}; trace=${JSON.stringify(trace)}`); }
    finally { owner.stop(); peer.stop(); }
  }, `/@fs${path.resolve(root, '../wasm_demo/pkg/verifier.js')}`);
  assert.deepEqual(pageErrors, []);
  const capture = result.requirementCaptures.at(-1);
  const requirements = capture.requirements.filter(requirement => requirement.type === 'private_open');
  const known = requirements.filter(requirement => requirement.card && requirement.card !== 'Hidden Card');
  const unknown = requirements.filter(requirement => !requirement.card || requirement.card === 'Hidden Card');
  assert.equal(known.length, 2, 'Serum Visions privately scried exactly two remaining library cards');
  assert.ok(unknown.length > 0, 'fetch also needs uncached remaining positions');
  const source = readFileSync(new URL('../src/hooks/peer-lobby/validation.js', import.meta.url), 'utf8');
  const start = source.indexOf('  async function collectZiffleRevealTokensBatch(');
  const end = source.indexOf('  async function buildLiveZiffleShuffleProofs(', start);
  const collectorSource = source.slice(start, end);
  const helper = readFileSync(new URL('./ziffle-reveal-token-collection.test.js', import.meta.url), 'utf8');
  const helperStart = helper.indexOf('function harness('), helperEnd = helper.indexOf('\ntest(', helperStart);
  const token = (player, cardPosition) => ({ player, cardPosition, publicKeyHex: `key-${player}`, tokenHex: 'synthetic-token', proofHex: 'synthetic-proof' });
  const gate = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };
  const harness = new Function('collectZiffleRevealTokenGroups', 'collectorSource', 'token', 'gate',
    `${helper.slice(helperStart, helperEnd)}; return harness;`)(collectZiffleRevealTokenGroups, collectorSource, token, gate);
  const position = requirement => Number(requirement.publicSlot ?? requirement.public_slot);
  const h = harness({ cached: [0, 1].flatMap(player => known.map(requirement => token(player, position(requirement)))) });
  Object.assign(h.ceremony, capture.ceremony);
  await h.collect(requirements.map(position), { command: capture.command, requirements: capture.requirements, seq: 99, actorIndex: 0 });
  assert.equal(h.requests.length, 1);
  const outbound = h.requests[0];
  assert.equal(outbound.actionAuthorization.requirements, undefined, 'local private requirement hints must never be serialized');
  for (const secret of known) {
    assert.ok(!JSON.stringify(outbound).includes(secret.id), 'no manifest-slot-bearing requirement ID');
    assert.ok(!JSON.stringify(outbound).includes(secret.commitment), 'no private manifest commitment');
  }
  assert.ok(known.every(secret => !outbound.cardPositions.includes(position(secret))), 'already-known positions are cached and were not requested');
  const evidence = { scenario: 'Marsh Flats shuffle, Serum Visions draw/scry2 keep both, Polluted Delta search',
    knownPrivateRequirements: known, requestedPositions: outbound.cardPositions, outboundPayload: outbound,
    trace: result.trace };
  // Opt-in evidence dump (e.g. for a player-mcp report); the assertions above are the test.
  if (process.env.ZIFFLE_WIRE_HINTS_EVIDENCE) {
    mkdirSync(path.dirname(process.env.ZIFFLE_WIRE_HINTS_EVIDENCE), { recursive: true });
    writeFileSync(process.env.ZIFFLE_WIRE_HINTS_EVIDENCE, JSON.stringify(evidence, null, 2));
  }
  console.log(JSON.stringify({ privateKnowledgeRetainedLocally: known.map(({ card, slot, publicSlot, id }) => ({ card, slot, publicSlot, id })), requestedPositions: outbound.cardPositions.length }));
});
