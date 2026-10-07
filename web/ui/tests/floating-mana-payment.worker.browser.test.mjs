import test from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('floating red mana enables and commits Kessig payment independently of workers', async () => {
  const root = fileURLToPath(new URL('..', import.meta.url));
  const pkg = fileURLToPath(new URL('../../wasm_demo/pkg/', import.meta.url));
  const vite = await createServer({ root, server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/payment-editor.html`);
    const result = await page.evaluate(async pkg => {
      const { default: init, WasmGame } = await import(`/@fs${pkg}ironsmith.js`);
      const { createLocalAnalysisJournal } = await import('/src/lib/local-analysis-replay.js');
      const module = await WebAssembly.compile(await (await fetch(`/@fs${pkg}engine_bg.wasm`)).arrayBuffer());
      await init({ engine: module, compiler: false, verifier: false });
      const journal = createLocalAnalysisJournal(new WasmGame(), 'floating-payment');
      const game = journal.game;
      const workers = [];
      const analyze = (request, kind) => new Promise((resolve, reject) => {
        const worker = new Worker('/src/workers/paymentOptionsWorker.js', { type: 'module' });
        workers.push(worker);
        worker.onerror = event => reject(new Error(event.message));
        worker.onmessage = ({ data }) => data.error ? reject(new Error(data.error)) : resolve(data.result);
        worker.postMessage({ token: workers.length, module, localReplay: journal.capture(), request, kind });
      });
      try {
        const sources = await Promise.all(['mountain', 'kessig-flamebreather'].map(async route =>
          (await fetch(`/cards/${route}.json`)).json()));
        const registered = JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(sources)));
        if (registered.failed.length) throw new Error(JSON.stringify(registered.failed));
        game.resetEmpty(['Alice', 'Bob'], 20);
        for (let i = 0; i < 2; i++) game.addCardToZone(0, 'Mountain', 'battlefield', true);
        game.addCardToZone(0, 'Kessig Flamebreather', 'hand', true);
        for (const seat of [0, 1]) for (let i = 0; i < 10; i++) game.addCardToZone(seat, 'Mountain', 'library', true);
        game.setDeferredPriorityAnalysis(true);
        game.setDeferredManaOptions(true);
        game.finishPuzzleSetup();
        for (let i = 0; i < 100; i++) {
          const state = game.uiState();
          if (state.active_player === 0 && /first.main/i.test(state.phase)) break;
          const action = state.decision?.actions?.find(a =>
            ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(a.kind));
          if (!action) throw new Error('Cannot advance fixture');
          game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
        }
        for (let i = 0; i < 2; i++) {
          const action = game.uiState().decision.actions.find(a => a.kind === 'activate_mana_ability');
          game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
        }
        const cast = game.uiState().decision.actions.find(a => a.kind === 'cast_spell');
        const payment = game.dispatch({ type: 'priority_action', action_ref: cast.action_ref }).mana_payment;
        // These values are captured before starting either auxiliary worker.
        const immediate = { canConfirm: payment.can_confirm, before: payment.pool_before.red,
          after: payment.pool_after_payment.red, allocations: payment.allocations.length };
        const request = game.exportManaPaymentOptionsRequest(payment.request_hash, payment.plan_id);
        const options = await analyze(request);
        await analyze('ranking', 'ranking');
        const paid = game.dispatch({ type: 'mana_payment', response: {
          action: 'confirm', plan_id: payment.plan_id, request_hash: payment.request_hash,
        } });
        return { ...immediate, options: options.activation_options, pool: paid.players[0].mana_pool.red,
          stack: paid.stack_objects.map(card => card.name) };
      } finally { for (const worker of workers) worker.terminate(); game.free(); }
    }, pkg);
    assert.equal(result.canConfirm, true);
    assert.equal(result.before, 2);
    assert.equal(result.after, 0);
    assert.equal(result.allocations, 2);
    assert.deepEqual(result.options, []);
    assert.equal(result.pool, 0);
    assert.ok(result.stack.includes('Kessig Flamebreather'));
  } finally { await browser.close(); await vite.close(); }
});
