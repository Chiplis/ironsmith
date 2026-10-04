import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import init, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import { createLocalAnalysisJournal, createLocalAnalysisReplica } from '../src/lib/local-analysis-replay.js';

await init({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });

function reachFirstMain(game) {
  for (let step = 0; step < 40; step++) {
    const state = game.uiState();
    if (state.active_player === 0 && /first.main/i.test(state.phase)) return;
    const pass = state.decision?.actions?.find(action => action.action_ref?.kind === "pass_priority");
    assert.ok(pass, `Expected priority while advancing fixture: ${JSON.stringify(state.decision)}`);
    game.dispatch({ type: "priority_action", action_ref: pass.action_ref });
  }
  throw new Error('Fixture did not reach first main phase');
}

for (const [name, text] of [
  ['Exhaust Replay Probe', 'Exhaust — {0}: You gain 1 life. (Activate each exhaust ability only once.)'],
  ['Turn Limit Replay Probe', '{0}: You gain 1 life. Activate only once each turn.'],
]) {
  test(`native analysis preserves ${name} ability usage through native replay`, async () => {
    const journal = createLocalAnalysisJournal(new WasmGame(), name), game = journal.game;
    game.initializeRuntimeIdentityOrigin(journal.capture().identityOrigin);
    const source = { canonicalName: name, group: { kind: 'single', name,
      block: `Mana Cost: {0}\nType: Artifact\n${text}` } };
    let exact;
    try {
      assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify([source]))).failed, []);
      game.resetEmpty(['Alice', 'Bob'], 20);
      game.addCardToZone(0, name, 'battlefield', true);
      for (const seat of [0, 1]) for (let card = 0; card < 5; card++) game.addCardToZone(seat, name, "library", true);
      game.finishPuzzleSetup();
      for (let i = 0; i < 4; i++) {
        const action = game.uiState().decision.actions.find(a => ['keep_opening_hand', 'continue_pregame', 'begin_game'].includes(a.action_ref?.kind));
        assert.ok(action);
        game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
      }
      reachFirstMain(game);
      const action = game.uiState().decision.actions.find(a => a.kind === 'activate_ability');
      assert.ok(action, 'the unused ability must be available');
      game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
      const payment = game.uiState().decision;
      assert.equal(payment.kind, 'mana_payment');
      game.dispatch({ type: 'mana_payment', response: { action: 'confirm', plan_id: payment.plan_id, request_hash: payment.request_hash } });
      const expected = game.uiState().decision;
      assert.equal(expected.actions.some(a => a.kind === 'activate_ability'), false);

      const replica = createLocalAnalysisReplica(() => new WasmGame());
      exact = await replica.hydrate(journal.capture());
      assert.deepEqual(exact.uiState().decision.actions, expected.actions);
      // Reuse the same snapshot after a preview changed its working state.
      exact.dispatch({ type: 'priority_action', action_ref: { kind: 'pass_priority' } });
      exact = await replica.hydrate(journal.capture());
      assert.deepEqual(exact.uiState().decision.actions, expected.actions);
    } finally { exact?.free(); game.free(); }
  });
}

test('real auxiliary workers retain a pending payment and preview targets from the same native state', async () => {
  const { readFileSync } = await import('node:fs');
  const { default: vm } = await import('node:vm');
  const journal = createLocalAnalysisJournal(new WasmGame(), 'auxiliary'), game = journal.game;
  const sources = [
    ['Payment Replay Probe', 'Mana Cost: {1}\nType: Artifact'],
    ['Target Replay Probe', 'Mana Cost: {0}\nType: Instant\nTarget player loses 1 life.'],
    ['Mana Replay Probe', 'Type: Land\n{T}: Add {R}.'],
  ].map(([name, block]) => ({ canonicalName: name, group: { kind: 'single', name, block } }));
  const children = [];
  class ChildGame extends WasmGame { constructor() { super(); children.push(this); } }
  async function run(workerName, data) {
    const messages = [], self = { postMessage: message => messages.push(message) };
    const code = readFileSync(new URL(`../src/workers/${workerName}.js`, import.meta.url), 'utf8')
      .replace(/^import[^\n]+\n/gm, '');
    vm.runInNewContext(code, { self, performance, setTimeout, WasmGame: ChildGame,
      initWasm: async () => {}, createLocalAnalysisReplica, castingMethodChoiceForAction: () => null });
    await self.onmessage({ data: { token: 1, id: 1, localReplay: journal.capture(), ...data } });
    assert.equal(messages.length, 1);
    assert.equal(messages[0].error, undefined, messages[0].error);
    return messages[0].result;
  }
  try {
    assert.deepEqual(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(sources))).failed, []);
    game.resetEmpty(['Alice', 'Bob'], 20);
    game.addCardToZone(0, 'Mana Replay Probe', 'battlefield', true);
    game.addCardToZone(0, 'Payment Replay Probe', 'hand', true);
    game.addCardToZone(0, 'Target Replay Probe', 'hand', true);
    for (const seat of [0, 1]) for (let card = 0; card < 5; card++) game.addCardToZone(seat, "Mana Replay Probe", "library", true);
    game.finishPuzzleSetup();
    for (let i = 0; i < 4; i++) {
      const action = game.uiState().decision.actions.find(a => ['keep_opening_hand', 'continue_pregame', 'begin_game'].includes(a.action_ref?.kind));
      game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
    }
    reachFirstMain(game);
    const actions = game.uiState().decision.actions;
    const targeted = actions.find(a => a.kind === 'cast_spell' && a.label.includes('Target Replay Probe'));
    assert.ok(targeted);
    const before = game.exportPublicAuditCheckpoint();
    const preview = await run('targetPreviewWorker', { actions: [targeted, targeted], perspective: 0 });
    assert.equal(preview.kind, 'targets');
    assert.equal(preview.requirements.length, 2, 'each preview resets to the canonical native snapshot');
    assert.deepEqual(game.exportPublicAuditCheckpoint(), before, 'preview leaves the original state intact');

    const paid = actions.find(a => a.kind === 'cast_spell' && a.label.includes('Payment Replay Probe'));
    assert.ok(paid);
    game.dispatch({ type: 'priority_action', action_ref: paid.action_ref });
    const payment = game.uiState().decision;
    assert.equal(payment.kind, 'mana_payment');
    const request = game.exportManaPaymentOptionsRequest(payment.request_hash, payment.plan_id);
    const expected = game.getPaymentActivationOptions(request);
    const actual = await run('paymentOptionsWorker', { request });
    delete actual.__payment_options_perf;
    assert.deepEqual(structuredClone(actual), expected);
    assert.ok(actual.activation_options.length > 0, 'the untapped land remains a payment option');
    assert.equal(game.uiState().decision.kind, 'mana_payment', 'alternatives do not consume the live transaction');
  } finally { for (const child of children) child.free(); game.free(); }
});
