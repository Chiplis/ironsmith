// Real proof coverage for simultaneous discard, public reveal choices, and private draw.
import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdir, writeFile } from 'node:fs/promises';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const reportRoot = path.resolve(root, '../../reports/player-mcp/zkp-protocol-fixes-2026-09-27');
const harnessId = '/tests/ziffle-batch-inline-harness.js';
const harnessSource = `
import React from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { usePeerLobbyConnections } from '/src/hooks/peer-lobby/connections.js';
import { usePeerLobbyAuditMaterial } from '/src/hooks/peer-lobby/audit-material.js';
import { usePeerLobbyCryptoResync } from '/src/hooks/peer-lobby/crypto-resync.js';
export function mountBatchServices(base, services, container) {
  function Harness() {
    Object.assign(services.current, usePeerLobbyConnections(base, services),
      usePeerLobbyAuditMaterial(base, services), usePeerLobbyCryptoResync(base, services));
    return null;
  }
  const root = createRoot(container);
  flushSync(() => root.render(React.createElement(Harness)));
  return root;
}
`;

for (const cardName of ['Windfall', 'Wheel of Fortune']) {
  test(`${cardName} synchronizes batch zone changes with independent hand knowledge`, { timeout: 120000 }, async t => {
    const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
      optimizeDeps: { entries: [], include: ['react', 'react-dom', 'react-dom/client'] },
      plugins: [{ name: 'batch-zone-test-harness', resolveId(id) { if (id === harnessId) return id; },
        load(id) { if (id === harnessId) return harnessSource; } }],
      server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
    await server.listen(); t.after(() => server.close());
    const browser = await chromium.launch(); t.after(() => browser.close());
    const page = await browser.newPage();
    const pageErrors = [];
    page.on('pageerror', error => pageErrors.push(error.message));
    await page.route('**/batch-zone-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="root0"></div><div id="root1"></div>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/batch-zone-test`);
    const result = await page.evaluate(async ({ verifierUrl, cardName }) => {
      const { mountBatchServices } = await import('/tests/ziffle-batch-inline-harness.js');
      const { buildPrivateDeckManifest, publicDeckManifest, publicCheckpointHash } = await import('/src/lib/multiplayer-audit.js');
      const { buildZiffleRuntimeManifest } = await import('/src/lib/ziffle-runtime-manifest.js');
      const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
      const api = await import(verifierUrl); await api.default();
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
      const engines = [createWorkerSession(), createWorkerSession()];
      const roots = [], contexts = [], states = [], trace = [];
      let stage = 'fixture', initial, spellStableId;
      const hashPair = async () => Promise.all(engines.map(async engine => publicCheckpointHash(await engine.call('exportPublicAuditCheckpoint'))));
      const summarize = checkpoint => ({ players: checkpoint.players.map(player => ({ id: player.id, hand: player.hand.length,
        library: checkpoint.objects.filter(object => object.hiddenCard?.owner === player.id && object.zone === 'library').length, graveyard: player.graveyard.length })),
        graveyard: checkpoint.objects.filter(object => object.zone === 'graveyard').map(object => ({ id: object.id, stableId: object.stableId,
          owner: object.hiddenCard?.owner, name: object.name, hiddenCard: object.hiddenCard })),
        spell: checkpoint.objects.find(object => object.stableId === spellStableId) });
      try {
        await Promise.all(engines.map(engine => engine.ready));
        const owner = engines[1], matchId = `batch-zone:${cardName}`;
        const deck = Array(61).fill('Mountain');
        deck[4] = cardName; deck[5] = 'Grizzly Bears'; deck[6] = 'Lightning Bolt'; deck[7] = 'Forest';
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
        const handNames = [['Grizzly Bears', 'Lightning Bolt', 'Forest'], [cardName, 'Grizzly Bears', 'Forest']];
        const decks = [deck.slice(), deck.slice()];
        const subjects = [1, 0].flatMap(seat => handNames[seat].map((name, index) => {
          const position = deckCount - 1 - index;
          const slot = reveals.find(reveal => reveal.cardPosition === position).originalSlot;
          decks[seat][slot] = name;
          return { seat, slot, position };
        }));
        const manifests = await Promise.all(decks.map((cards, seat) => buildPrivateDeckManifest({ matchId, owner: seat, deck: cards })));
        const ceremony = { owner: 1, deckCount, context, keys, steps, deckHash: verified.deckHash, tokens, reveals };
        const setupCall = async (method, ...args) => {
          await engines[0].call(method, ...args);
          return engines[1].call(method, ...args);
        };
        await engines[0].call('setPerspective', 0);
        await owner.call('setPerspective', 1);
        let state = await setupCall('startMatch', { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1,
          format: 'normal', startingPlayer: 1, openingHandSize: 0, decks: [[], []], publicDecklists: decks,
          hiddenDeckManifests: manifests.map((manifest, seat) => buildZiffleRuntimeManifest(manifest, { ...ceremony, owner: seat })) });
        for (let index = 0; index < 30 && state.phase !== 'first main phase'; index++) {
          const action = state.decision?.actions?.find(item => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(item.action_ref?.kind));
          if (!action) throw new Error(`Unexpected pregame ${JSON.stringify(state.decision)}`);
          state = await setupCall('dispatch', { type: 'priority_action', action_ref: action.action_ref });
        }
        if (state.phase !== 'first main phase') throw new Error('Main phase missing');
        // Draw each three-card hand on both engines, opening its identities
        // only on the owning client before the simultaneous discard spell.
        for (const subject of subjects) {
          const session = engines[subject.seat];
          const metadata = await session.call('getHiddenCardState');
          const object = metadata.objects.find(item => item.hiddenCard?.owner === subject.seat && item.hiddenCard.slot === subject.position);
          if (!object) throw new Error('Missing committed hand subject');
          const secret = manifests[subject.seat].slotSecrets.find(entry => entry.slot === subject.slot);
          await session.call('revealHiddenPosition', { owner: subject.seat, objectId: object.id, position: subject.position,
            originalSlot: subject.slot, cardName: secret.card, commitment: secret.commitment,
            positionCommitment: `ziffle:${ceremony.deckHash}:${subject.position}` });
          if (subject.seat === 1 && subject === subjects.find(entry => entry.seat === 1)) spellStableId = object.stableId;
          await setupCall('drawCard', subject.seat);
          const drawn = await session.call('getHiddenCardState');
          subject.id = drawn.objects.find(item => item.stableId === object.stableId).id;
        }
        for (let index = 0; index < 3; index++) await setupCall('addCardToZone', 1, cardName === 'Wheel of Fortune' ? 'Mountain' : 'Island', 'battlefield', true);
        for (const seat of [0, 1]) {
          states[seat] = await engines[seat].call('uiState');
          const game = new Proxy({}, { get: (_, method) => method.startsWith('ziffle')
            ? async input => api[method](input) : (...args) => engines[seat].call(method, ...args) });
          const refs = { gameRef: { current: game }, stateRef: { current: states[seat] },
            multiplayerRef: { current: { localPlayerIndex: seat, players: manifests.map((manifest, index) => ({ index, deckAuditManifest: publicDeckManifest(manifest) })) } },
            verifiedAuditOpeningsRef: { current: new Set() }, actionHistoryRef: { current: [] },
            matchStartPayloadRef: { current: { auditMatchId: matchId, deckAuditManifests: manifests.map(publicDeckManifest), players: [{ index: 0 }, { index: 1 }] } },
            privateDeckManifestsRef: { current: new Map([[`${matchId}:${seat}`, manifests[seat]]]) },
            liveZiffleCeremoniesRef: { current: new Map([0, 1].map(index => [index, { ...ceremony, owner: index }])) },
            localZiffleCeremonyLookupRef: { current: new Map([['initial', ceremony]]) },
            setState() {}, setStatus() {}, setMultiplayer() {} };
          const base = new Proxy(refs, { get: (object, key) => object[key] ||= { current: new Map() } });
          const services = { current: { collectZiffleRevealTokens: async (_selected, position) => tokens.filter(token => token.cardPosition === position),
            collectZiffleRevealTokensBatch: async (_selected, selected) => tokens.filter(token => selected.includes(token.cardPosition)) } };
          roots.push(mountBatchServices(base, services, document.getElementById(`root${seat}`)));
          contexts[seat] = { refs, services: services.current };
        }
        const checkpoints = await Promise.all(engines.map(engine => engine.call('getHiddenCardState')));
        initial = { hashes: await hashPair(), hands: checkpoints.map((checkpoint, seat) => [0, 1].map(ownerSeat =>
          checkpoint.objects.filter(object => object.hiddenCard?.owner === ownerSeat && object.zone === 'hand').map(object => ({ name: object.name, knownTo: seat })))) };
        const refresh = async () => { for (const seat of [0, 1]) {
          states[seat] = await engines[seat].call('uiState'); contexts[seat].refs.stateRef.current = states[seat];
        } };
        const buildPublic = async (command, requirements) => {
          const output = [];
          for (const { services } of contexts) {
            if (command) output.push(...await services.buildLocalOpeningsForCommand(command, requirements));
            output.push(...await services.buildLocalRequirementOpeningsForRequirements(requirements, { forceZiffleOpeningProof: true }));
          }
          return [...new Map(output.map(opening => [`${opening.owner}:${opening.slot}:${opening.commitment}:${opening.timing || 'pre'}`, opening])).values()];
        };
        const applyPublic = async (openings, timing, command = null) => {
          for (const { services } of contexts) await services.revealAuditOpenings(openings, { timing, command, updateState: false, previewInspector: false });
          await refresh();
        };
        const hydratePrivate = async requirements => {
          const counts = [];
          for (const { services } of contexts) {
            const openings = await services.privateOpeningsForLocalViewer(requirements, {}, { forceZiffleOpeningProof: true });
            await services.revealPrivateOpeningsForInjection(openings, { updateState: false, previewInspector: false });
            counts.push(openings.length);
          }
          await refresh(); return counts;
        };
        const compactRequirements = requirements => requirements.map(requirement => ({ type: requirement.type, owner: requirement.owner, viewer: requirement.viewer,
          slot: requirement.slot, objectId: requirement.objectId, from: requirement.from, to: requirement.to, card: requirement.card }));
        const dispatchBoth = async command => {
          const index = trace.length, entry = { index, command, beforeDecisions: states.map(value => ({ kind: value.decision?.kind, player: value.decision?.player, reason: value.decision?.reason })) };
          trace.push(entry);
          const actor = states[1].decision?.player ?? 1;
          stage = `${index}:preview`;
          let requirements;
          try { requirements = await engines[actor].call('previewCryptoRequirements', command); }
          catch (error) {
            entry.previewErrors = [null, null];
            entry.previewErrors[actor] = String(error.message);
            try { await engines[1 - actor].call('previewCryptoRequirements', command); }
            catch (peerError) { entry.previewErrors[1 - actor] = String(peerError.message); }
            // Preview executes a cloned state. Diagnose the same command on
            // each still-unmodified live worker before terminating the probe.
            entry.beforeDirectDispatchHashes = await hashPair();
            const direct = await Promise.allSettled(engines.map(engine => engine.call('dispatch', command)));
            entry.directDispatchErrors = direct.map(outcome => outcome.status === 'rejected' ? String(outcome.reason.message) : null);
            entry.afterDirectDispatchHashes = await hashPair();
            throw new Error(`${entry.previewErrors.every(Boolean) ? 'Both previews rejected' : 'Only one preview rejected'}: ${JSON.stringify(entry.previewErrors)}`);
          }
          entry.requirements = compactRequirements(requirements);
          stage = `${index}:pre-private-opening`;
          entry.prePrivate = await hydratePrivate(requirements);
          stage = `${index}:public-opening-build`;
          const openings = await buildPublic(command, requirements);
          entry.openings = openings.map(opening => ({ owner: opening.owner, slot: opening.slot, card: opening.card, timing: opening.timing, origin: opening.originPosition }));
          stage = `${index}:pre-public-opening`;
          await applyPublic(openings, 'pre', command);
          stage = `${index}:dispatch`;
          const outcomes = await Promise.allSettled(engines.map(engine => engine.call('dispatch', command)));
          entry.dispatchErrors = outcomes.map(outcome => outcome.status === 'rejected' ? String(outcome.reason.message) : null);
          if (entry.dispatchErrors.some(Boolean)) throw new Error(`${entry.dispatchErrors.every(Boolean) ? 'Both engines rejected' : 'Only one engine rejected'}: ${JSON.stringify(entry.dispatchErrors)}`);
          for (const seat of [0, 1]) { states[seat] = outcomes[seat].value; contexts[seat].refs.stateRef.current = states[seat]; }
          const postRequirements = outcomes.flatMap(outcome => outcome.value.crypto_requirements || outcome.value.cryptoRequirements || []);
          entry.postRequirements = compactRequirements(postRequirements);
          stage = `${index}:post-public-opening`;
          await applyPublic(openings, 'post');
          const afterOpenings = await buildPublic(null, [...requirements, ...postRequirements]);
          await applyPublic(afterOpenings, 'post');
          stage = `${index}:post-private-opening`;
          entry.postPrivate = await hydratePrivate(postRequirements);
          stage = `${index}:public-hash`;
          entry.hashes = await hashPair();
          entry.afterDecisions = states.map(value => ({ kind: value.decision?.kind, player: value.decision?.player, reason: value.decision?.reason }));
          if (entry.hashes[0] !== entry.hashes[1]) throw new Error('Public checkpoint hashes differ');
        };
        const action = states[1].decision.actions.find(item => item.action_ref?.kind === 'cast_spell' && Number(item.object_id) === subjects[0].id);
        if (!action) throw new Error('Batch spell not playable');
        await dispatchBoth({ type: 'priority_action', action_ref: action.action_ref, object_id: subjects[0].id });
        for (let index = 0; index < 24; index++) {
          const checkpoint = await engines[1].call('getHiddenCardState');
          if (states[1].decision?.kind === 'priority' && checkpoint.objects.find(object => object.stableId === spellStableId)?.zone === 'graveyard') break;
          const actor = states[1].decision?.player ?? 1;
          const decision = states[actor].decision;
          if (decision?.kind === 'mana_payment') {
            await dispatchBoth({ type: 'mana_payment', response: { action: 'confirm', plan_id: decision.plan_id, request_hash: decision.request_hash } });
          } else if (decision?.kind === 'priority') {
            const pass = decision.actions.find(item => item.action_ref?.kind === 'pass_priority');
            if (!pass) throw new Error('Pass action absent');
            await dispatchBoth({ type: 'priority_action', action_ref: pass.action_ref });
          } else if (decision?.kind === 'select_objects') {
            const chosen = decision.candidates.filter(candidate => candidate.legal);
            if (chosen.length !== decision.min) throw new Error(`Unexpected optional discard choice: ${JSON.stringify(decision)}`);
            await dispatchBoth({ type: 'select_objects', object_ids: chosen.map(candidate => candidate.id),
              object_hidden_refs: chosen.map(candidate => ({ owner: candidate.hidden_ref.owner, zone: candidate.hidden_ref.zone,
                public_slot: candidate.hidden_ref.public_slot, public_commitment: candidate.hidden_ref.public_commitment })) });
          } else if (decision?.kind === 'select_options' && decision.reason === 'Ordering') {
            await dispatchBoth({ type: 'select_options', option_indices: decision.options.map(option => option.index) });
          } else throw new Error(`Unsupported fixture decision ${JSON.stringify(decision)}`);
        }
        const final = await Promise.all(engines.map(engine => engine.call('getHiddenCardState')));
        return { ok: true, cardName, initial, trace, final: final.map(summarize), finalHands: final.map((checkpoint, seat) =>
          [0, 1].map(ownerSeat => checkpoint.objects.filter(object => object.hiddenCard?.owner === ownerSeat && object.zone === 'hand')
            .map(object => ({ name: object.name, originalCardName: object.originalCardName, knownTo: seat })))), hashes: await hashPair() };
      } catch (error) {
        return { ok: false, cardName, stage, error: String(error.message), initial, trace,
          final: await Promise.all(engines.map(async engine => summarize(await engine.call('getHiddenCardState')))), hashes: await hashPair() };
      } finally { roots.forEach(reactRoot => reactRoot.unmount()); engines.forEach(engine => engine.worker.terminate()); }
    }, { cardName, verifierUrl: `/@fs/${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
    await mkdir(reportRoot, { recursive: true });
    await writeFile(path.join(reportRoot, `${cardName.toLowerCase().replaceAll(' ', '-')}-evidence.json`), JSON.stringify(result, null, 2));
    assert.deepEqual(pageErrors, []);
    assert.equal(result.ok, true, `${result.stage}: ${result.error}`);
    assert.equal(result.initial.hashes[0], result.initial.hashes[1]);
    for (const seat of [0, 1]) {
      assert.ok(result.initial.hands[seat][seat].every(card => card.name !== 'Hidden Card'));
      assert.ok(result.initial.hands[seat][1 - seat].every(card => card.name === 'Hidden Card'));
      assert.equal(result.final[seat].spell.zone, 'graveyard');
      assert.deepEqual(result.final[seat].players.map(player => player.hand), cardName === 'Windfall' ? [3, 3] : [7, 7]);
      assert.deepEqual(result.final[seat].players.map(player => player.graveyard), [3, 3]);
      assert.ok(result.finalHands[seat][seat].every(card => card.name !== 'Hidden Card'), 'the owner hydrates its privately drawn cards');
      assert.ok(result.finalHands[seat][1 - seat].every(card => card.name === 'Hidden Card' && !card.originalCardName), 'opposing drawn cards remain private');
      assert.ok(result.final[seat].graveyard.every(card => card.name !== 'Hidden Card'), 'discarded cards are publicly known on both peers');
    }
    assert.equal(result.hashes[0], result.hashes[1]);
    t.diagnostic(JSON.stringify({ cardName, commands: result.trace.length, hashesMatch: true,
      preparedOpenings: result.trace.reduce((sum, entry) => sum + entry.openings.length, 0),
      privateOpenings: result.trace.reduce((sum, entry) => sum + entry.prePrivate.reduce((a, b) => a + b, 0) + entry.postPrivate.reduce((a, b) => a + b, 0), 0) }));
  });
}
