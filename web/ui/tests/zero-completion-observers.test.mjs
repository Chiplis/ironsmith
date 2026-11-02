import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { costProgramReferences } from '../src/lib/reminder-cost-programs.js';
const definition = { costProgram: true, forms: { eventVerb: 'collect backing', eventLabel: 'collect backing' }, paymentReferences: [{ subject: 'backing', participle: 'collected', countUnit: 'color' }] };
test('exact sourced positive-bound observers retain their bound and source qualifier', () => {
  const source = JSON.parse(readFileSync(new URL('../../../reports/cards/lackey-audit-2026-11-01/msem-complete-source.json', import.meta.url)));
  const cards = source.cards;
  for (const name of ['Golna, Shah', 'Spoils of the Stinging King', 'Withered Sentinel']) {
    const card = cards.find(card => card.key === `${name}_G5L`);
    assert.equal(card?.cardName, name, 'lookup must use the exact physical source key');
    assert.ok(card, `exact source missing: ${name}`);
    const original = card.rulesText.split('\n').find(line => /^Whenever you collect backing from one or more colors/.test(line));
    assert.ok(original, name);
    const rewritten = costProgramReferences(original, definition);
    assert.match(rewritten, /^Whenever you perform the collect backing action with an amount of 1 or more/);
    if (name === 'Withered Sentinel') assert.match(rewritten, /while Withered Sentinel is in your graveyard,/);
    assert.ok(card.rulesText.includes(original));
    if (name === 'Withered Sentinel') {
      assert.equal(card.typeLine, 'Creature — Zombie Soldier');
      assert.ok(card.rulesText.startsWith('Withered Sentinel enters tapped.\n'), 'complete source keeps its sibling ability');
    }
  }
});
test('threshold aliases use explicit generic metadata; an unbounded observer does not acquire a minimum', () => {
  const other = { ...definition, forms: { eventVerb: 'assess', eventLabel: 'assess' }, paymentReferences: [{ countUnit: 'charge' }] };
  assert.equal(costProgramReferences('Whenever you assess from two or more charges, draw that number of cards.', other), 'Whenever you perform the assess action with an amount of 2 or more, draw that many cards.');
  assert.equal(costProgramReferences('Whenever you assess, draw a card.', other), 'Whenever you assess, draw a card.');
  assert.equal(costProgramReferences('Whenever you assess from one or more colors, draw a card.', other), 'Whenever you assess from one or more colors, draw a card.');
});
