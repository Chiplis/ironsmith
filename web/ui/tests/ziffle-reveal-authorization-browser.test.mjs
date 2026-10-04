import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { authorizationHarness, shuffleProof } from './ziffle-reveal-authorization-harness.mjs';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('real worker draw, fetch, scry and mulligan requirements authorize exact reveal positions', { timeout: 120000 }, async t => {
  const vite = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await vite.listen(); t.after(() => vite.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/authorization-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Authorization engine regression</title>' }));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/authorization-test`);
  const captures = await page.evaluate(async () => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    function createWorker() {
      const decoder = createSnapshotDecoder(), pending = new Map(), analyses = new Map(), analysisWaiters = new Map();
      const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
      let id = 0;
      const ready = new Promise((resolve, reject) => {
        worker.onerror = error => reject(new Error(error.message));
        worker.onmessage = ({ data }) => {
          if (data.type === 'error') return reject(new Error(data.error.message));
          if (data.type === 'ready') return resolve();
          if (data.type === 'priorityAnalysis') { if (data.decision.analysis_complete !== true) return; analyses.set(data.revision, data.decision); analysisWaiters.get(data.revision)?.(data.decision); return; }
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
        const decision = analyses.get(revision) || await new Promise(resolve => analysisWaiters.set(revision, resolve));
        analysisWaiters.delete(revision);
        return { ...value, decision };
      });
      worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
      return { ready, call, stop: () => worker.terminate() };
    }
    const workers = [createWorker(), createWorker()];
    const result = [];
    try {
      await Promise.all(workers.map(worker => worker.ready));
      for (let seat = 0; seat < workers.length; seat++) {
        const worker = workers[seat];
        const config = { playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1, format: 'normal',
          startingPlayer: 0, openingHandSize: 6, decks: [Array(60).fill('Island'), Array(60).fill('Mountain')],
          publicDecklists: [Array(60).fill('Island'), Array(60).fill('Mountain')],
          hiddenDeckManifests: [0, 1].map(owner => ({ owner, deckCount: 60, commitmentRoot: `ziffle:deck-${owner}`,
            slotCommitments: Array.from({ length: 60 }, (_, slot) => ({ slot, commitment: `ziffle:deck-${owner}:${slot}` })) })) };
        await worker.call('setPerspective', seat);
        for (const scenario of ['mulligan', 'draw', 'fetch', 'scry']) {
          let state = await worker.call('startMatch', config);
          let acted = false, spellId = null;
          if (scenario === 'fetch') spellId = Number(await worker.call('addCardToZone', 0, 'Evolving Wilds', 'battlefield', true));
          if (scenario === 'scry') {
            spellId = Number(await worker.call('addCardToZone', 0, 'Preordain', 'hand', true));
            await worker.call('addCardToZone', 0, 'Island', 'battlefield', true);
          }
          state = await worker.call('uiState');
          for (let step = 0; step < 160; step++) {
            const actions = state.decision?.actions || [];
            let action = !acted && scenario === 'mulligan' ? actions.find(value => value.action_ref?.kind === 'take_mulligan') : null;
            if (!action && !acted && scenario === 'fetch') action = actions.find(value => value.action_ref?.kind === 'activate_ability' && Number(value.object_id ?? value.action_ref.source ?? value.action_ref.source_id) === Number(spellId));
            if (!action && !acted && scenario === 'scry') action = actions.find(value => value.action_ref?.kind === 'cast_spell' && Number(value.action_ref.spell_id) === Number(spellId));
            if (action) acted = true;
            action ||= actions.find(value => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(value.action_ref?.kind));
            let command;
            if (action) command = { type: 'priority_action', action_ref: action.action_ref };
            else if (state.decision?.kind === 'mana_payment') command = { type: 'mana_payment', response: { action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash } };
            else if (['attackers', 'blockers'].includes(state.decision?.kind)) command = { type: state.decision.kind === 'attackers' ? 'declare_attackers' : 'declare_blockers', declarations: [] };
            else throw new Error(`${scenario}: ${JSON.stringify(state.decision)}`);
            const decision = state.decision;
            const checkpoint = await worker.call('getHiddenCardState');
            const requirements = await worker.call('previewCryptoRequirements', command);
            state = await worker.call('dispatch', command);
            const matches = scenario === 'mulligan' ? requirements.some(value => value.type === 'verifiable_shuffle')
              : scenario === 'draw' ? state.turn_number >= 2 && requirements.some(value => value.type === 'private_open')
              : requirements.some(value => value.type === 'private_view_window');
            if (matches) { result.push({ seat, scenario, requirements, checkpoint, command, decision }); break; }
            if (step === 159) throw new Error(`No ${scenario} requirements`);
          }
        }
      }
      return result;
    } finally { workers.forEach(worker => worker.stop()); }
  });
  assert.equal(captures.length, 8);
  for (const capture of captures) {
    const h = authorizationHarness({ requirements: capture.requirements, checkpoint: capture.checkpoint, decisionPlayer: capture.decision.player });
    h.message.actionAuthorization.command = capture.command;
    h.message.actionAuthorization.actorIndex = capture.decision.player;
    const exactOpenings = capture.requirements.filter(value => value.type === 'private_open');
    assert.ok(exactOpenings.length > 0, `${capture.scenario} captures exact owner openings`);
    for (const requirement of exactOpenings) {
      const originSlot = requirement.originSlot ?? requirement.origin_slot;
      const originCommitment = requirement.originCommitment ?? requirement.origin_commitment;
      assert.ok(Number.isInteger(originSlot), `${capture.scenario} seat ${capture.seat} preserves genesis slot`);
      assert.equal(originCommitment, `ziffle:deck-${requirement.owner}:${originSlot}`);
      const originCeremony = { owner: requirement.owner, deckCount: 60, deckHash: `deck-${requirement.owner}` };
      assert.equal(h.direct([requirement], requirement.viewer, requirement.owner, [originSlot], originCeremony), true);
      assert.equal(h.direct([requirement], requirement.viewer, requirement.owner, [(originSlot + 1) % 60], originCeremony), false);
      assert.equal(h.direct([requirement], 1 - Number(requirement.viewer), requirement.owner, [originSlot], originCeremony), false);
    }
    if (capture.scenario === 'mulligan') {
      const requirement = capture.requirements.find(value => value.type === 'verifiable_shuffle');
      h.message.actionAuthorization.shuffleProofs = [shuffleProof(requirement)];
      const before = requirement.beforeOrder || requirement.before_order, after = requirement.afterOrder || requirement.after_order;
      const ceremony = { owner: requirement.owner, deckCount: after.length, deckHash: 'shuffled', beforeOrder: before, afterOrder: after,
        context: `match:action:8:shuffle:${requirement.id}:${requirement.owner}:library` };
      const positions = Array.from({ length: after.length - requirement.count }, (_, index) => requirement.count + index);
      assert.ok(positions.length > 0, 'mulligan produces an owner hand tail');
      h.message.actionAuthorization.requesterIndex = requirement.owner;
      assert.equal(await h.authorize(h.message, requirement.owner, requirement.owner, positions, ceremony), true);
      assert.equal(h.direct(capture.requirements, requirement.owner, requirement.owner, positions, ceremony, 8), true);
      assert.equal(h.direct(capture.requirements, requirement.owner, requirement.owner, [0], ceremony, 8), false);
    } else {
      const openings = capture.requirements.filter(value => value.type === 'private_open');
      assert.ok(openings.length > 0, `${capture.scenario} produces exact openings`);
      for (const opening of openings) {
        const commitment = opening.public_commitment || opening.publicCommitment || opening.commitment;
        const position = Number(commitment.slice(commitment.lastIndexOf(':') + 1));
        const ceremony = { owner: opening.owner, deckCount: 60, deckHash: `deck-${opening.owner}` };
        h.message.actionAuthorization.requesterIndex = opening.viewer;
        assert.equal(await h.authorize(h.message, opening.viewer, opening.owner, [position], ceremony), true);
        assert.equal(h.direct(capture.requirements, opening.viewer, opening.owner, [position], ceremony), true, `${capture.scenario} seat ${capture.seat}`);
        assert.equal(h.direct(capture.requirements, opening.viewer, opening.owner, [position], { ...ceremony, deckHash: 'later' }), false);
      }
      if (capture.scenario === 'fetch') assert.ok(openings.length >= 50, 'fetch authorizes the real library window');
      if (capture.scenario === 'scry') assert.equal(openings.length, 2);
    }
  }
});
