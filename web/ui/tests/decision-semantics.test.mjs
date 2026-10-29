import test from 'node:test';
import assert from 'node:assert/strict';
import { isPaymentDecision, isSpellCastFlowDecision, isOrderingDecision, isDistributionDecision, isCounterOptionsDecision } from '../src/lib/decision-semantics.js';
import { tryBuildAutoResolveCommand } from '../src/lib/auto-resolve-decision.js';
import { defaultOptionSelection } from '../src/lib/opponent-decision.js';
import { decisionKey } from '../src/lib/decision-key.js';
const menu = (extra = {}) => ({ kind: 'select_options', min: 1, max: 1, options: [{ index: 7, description: 'Pay No Heed', legal: true }], ...extra });

test('card names and mode text do not become payment or casting controls', () => {
  for (const choice_kind of [undefined, 'options', 'modes']) {
    for (const name of ['Pay No Heed', 'Pay Your Debts', 'Cast Down', 'Play with Fire']) {
      const decision = menu({ choice_kind, description: 'Choose a card name', options: [{ index: 7, description: name }] });
      assert.equal(isPaymentDecision(decision), false);
      assert.equal(isSpellCastFlowDecision(decision), false);
    }
  }
  assert.equal(isPaymentDecision(menu({ reason: 'Modal choice', description: 'Choose mode for Test', options: [{ description: 'Pay {2}. Draw a card.' }] })), false);
  for (const description of ['Cast this spell', 'Choose which spell to cast', 'Choose a casting price']) {
    assert.equal(tryBuildAutoResolveCommand(menu({ description })), null);
  }
  for (const description of ['Pay {2}', 'Choose how to pay pip 1 of 2', 'Choose payment method']) {
    assert.equal(isPaymentDecision(menu({ description })), true);
    assert.equal(tryBuildAutoResolveCommand(menu({ description })), null);
  }
});

test('typed choice kinds override misleading presentation metadata', () => {
  const decision = menu({ choice_kind: 'options', reason: 'Order triggers', description: 'Choose the replacement word' });
  assert.equal(isOrderingDecision(decision), false);
  assert.equal(isDistributionDecision({ ...decision, reason: 'Distribute' }), false);
  assert.equal(isCounterOptionsDecision({ ...decision, reason: 'Remove counters' }), false);
  assert.equal(isOrderingDecision(menu({ choice_kind: 'order', reason: null })), true);
  assert.equal(isDistributionDecision(menu({ choice_kind: 'distribution', reason: null })), true);
  assert.notEqual(decisionKey(decision), decisionKey({ ...decision, choice_kind: 'order' }));
});

test('automatic submission respects option weights and availability', () => {
  for (const option of [{ point_cost: 2 }, { point_cost: 0 }, { max_count: 0 }, { legal: false }]) {
    assert.equal(tryBuildAutoResolveCommand(menu({ options: [{ index: 7, legal: true, ...option }] })), null);
  }
  assert.deepEqual(tryBuildAutoResolveCommand(menu({ description: 'Choose a card name' }))?.cmd,
    { type: 'select_options', option_indices: [7] });
  assert.equal(tryBuildAutoResolveCommand(menu({ min: 0 })), null);
});

test('opponent allocation follows the distribution contract, not quoted text', () => {
  const options = [{ index: 2, repeatable: true, max_count: 4 }];
  assert.deepEqual(defaultOptionSelection(menu({ choice_kind: 'options', min: 0, max: 4,
    description: 'A mode mentioning assign exactly 4 total', options })), []);
  assert.deepEqual(defaultOptionSelection(menu({ choice_kind: 'distribution', min: 0, max: 4,
    description: 'Translated prompt', options })), [2, 2, 2, 2]);
});

test('empty blocker choices auto-confirm but any legal blocker remains manual', () => {
  assert.deepEqual(tryBuildAutoResolveCommand({ kind: 'blockers', blocker_options: [] })?.cmd,
    { type: 'declare_blockers', declarations: [] });
  assert.ok(tryBuildAutoResolveCommand({ kind: 'blockers', blocker_options: [{ valid_blockers: [] }] }));
  assert.equal(tryBuildAutoResolveCommand({ kind: 'blockers', blocker_options: [{ valid_blockers: [{ id: 12 }] }] }), null);
  assert.equal(tryBuildAutoResolveCommand({ kind: 'blockers' }), null);
});

test('empty attack and block declarations answer themselves; real choices do not', () => {
  assert.deepEqual(tryBuildAutoResolveCommand({ kind: 'attackers', player: 0, attacker_options: [] })?.cmd,
    { type: 'declare_attackers', declarations: [] });
  assert.equal(tryBuildAutoResolveCommand({ kind: 'attackers', player: 0, attacker_options: [{ creature: 3, valid_targets: [] }] }), null);
  assert.deepEqual(tryBuildAutoResolveCommand({ kind: 'blockers', player: 1, blocker_options: [{ attacker: 3, valid_blockers: [] }] })?.cmd,
    { type: 'declare_blockers', declarations: [] });
});
