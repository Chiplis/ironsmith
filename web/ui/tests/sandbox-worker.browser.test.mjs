import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('sandbox starts through the authoritative worker with card loading and real permissions', { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/sandbox-worker-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Sandbox test</title>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/sandbox-worker-test`);
  const result = await page.evaluate(async () => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const decoder = createSnapshotDecoder(), pending = new Map();
    const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
    let id = 0, patches = 0, full = 0;
    const analyses = new Map();
    const ready = new Promise((resolve, reject) => {
      worker.onerror = error => reject(new Error(error.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'error') { reject(new Error(data.error.message)); return; }
        if (data.type === 'ready') { resolve(data); return; }
        if (data.type === 'priorityAnalysis') analyses.set(data.revision, data);
        if (data.type === 'priorityAnalysisError') analyses.set(data.revision, data);
        if (data.type !== 'result') return;
        const request = pending.get(data.id); if (!request) return;
        pending.delete(data.id);
        if (!data.ok) { request.reject(new Error(data.error.message)); return; }
        if (data.snapshot) {
          if ('full' in data.snapshot) full++; else patches++;
          request.resolve(decoder.decode(data.snapshot));
        } else request.resolve(data.result);
      };
    });
    const call = (method, ...args) => new Promise((resolve, reject) => {
      pending.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
    });
    worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
    try {
      await ready;
      const { buildSandboxPlan, sandboxerDraft } = await import('/src/lib/sandbox.js');
      const draft = { layout: 'single', hasFuse: false, faces: [{ name: 'Sandbox Worker Bear', manaCost: '{7}{G}', cardTypes: ['Creature'], oracleText: '', power: '2', toughness: '2' }] };
      const plan = buildSandboxPlan({ selections: [{ id: 'bear', draft, copies: 4 }], testedId: 'bear', deck: Array(56).fill('Forest'), options: { freeCasting: true, graveyard: true, exile: true, unlimitedLands: true } }, () => 0.999);
      await call('startSandbox', plan);
      let state = await call('uiState');
      const setup = state;
      for (let step = 0; step < 30; step++) {
        if ((state.decision?.actions || []).some(action => action.action_ref?.kind === 'cast_spell')) break;
        const action = (state.decision?.actions || []).find(action => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(action.action_ref?.kind));
        if (!action) break;
        state = await call('dispatch', { type: 'priority_action', action_ref: action.action_ref });
      }
      // Land actions must be usable in the immediate snapshot, before a
      // background affordability result arrives (and after each land play).
      for (let play = 0; play < 2; play++) {
        const land = state.decision?.actions.find(action => action.action_ref?.kind === 'play_land');
        if (!land) throw new Error(`Land play ${play + 1} missing from immediate menu`);
        if (land.payment_proven !== true) throw new Error('A land play must not wait for mana analysis');
        const landId = land.action_ref.land_id;
        state = await call('dispatch', { type: 'priority_action', action_ref: land.action_ref });
        if (!state.players[0].battlefield.some(card => Number(card.id) === Number(landId)
          || card.member_ids?.some(id => Number(id) === Number(landId)))) {
          throw new Error('Land did not enter the battlefield');
        }
      }
      for (let wait = 0; wait < 600 && !analyses.get(state.__priority_revision)?.decision?.analysis_complete; wait++) {
        const error = analyses.get(state.__priority_revision)?.error;
        if (error) throw new Error(JSON.stringify(error));
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      const analysis = analyses.get(state.__priority_revision);
      if (analysis?.decision) state = { ...state, decision: analysis.decision };
      const landActions = (state.decision?.actions || []).filter(action => action.action_ref?.kind === 'play_land');
      const unlimitedState = state;
      await call('startSandbox', { ...plan, sandboxer: sandboxerDraft({ freeCasting: true }) });
      state = await call('uiState');
      for (let step = 0; step < 30; step++) {
        if (state.phase === 'first main phase') break;
        const action = (state.decision?.actions || []).find(action => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(action.action_ref?.kind));
        if (!action) throw new Error('Could not reach main phase');
        state = await call('dispatch', { type: 'priority_action', action_ref: action.action_ref });
      }
      const normalLand = state.decision?.actions.find(action => action.action_ref?.kind === 'play_land');
      if (!normalLand) throw new Error('Normal first land play missing from immediate menu');
      state = await call('dispatch', { type: 'priority_action', action_ref: normalLand.action_ref });
      if (state.decision?.actions.some(action => action.action_ref?.kind === 'play_land')) {
        throw new Error('Normal one-land limit was bypassed');
      }
      return { setup, state: unlimitedState, patches, full, landActions };
    } finally { worker.terminate(); }
  });
  assert.equal(result.setup.players.length, 2);
  assert.equal(result.setup.players[0].hand_cards.length, 7);
  assert.ok(result.setup.players[0].hand_cards.some(card => card.name === 'Sandbox Worker Bear'));
  assert.ok(result.landActions.length > 0, `lands must be playable: ${JSON.stringify(result.state)}`);
  const testCard = result.setup.players[0].hand_cards.find(card => card.name === 'Sandbox Worker Bear');
  assert.ok(result.state.decision?.actions.some(action => action.action_ref?.kind === 'cast_spell' && Number(action.action_ref.spell_id) === Number(testCard.id)), 'the test card can be cast with an empty mana pool');
});
