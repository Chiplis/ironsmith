import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src/hooks/peer-lobby/validation.js', import.meta.url), 'utf8');
const helpers = source.slice(source.indexOf('  function isInspectorOnlyViewedCards('),
  source.indexOf('  async function ziffleRevealTokenOptionsForLocalHandReveal('));
const { preserveViewedCardsFromHint } = new Function(helpers + '\nreturn { preserveViewedCardsFromHint };')();
const hand = { source: 129, subject: 1, zone: 'hand', visibility: 'public',
  viewer: 0, inspector_only: false, acknowledged: false,
  description: "Reveal that player's hand", card_ids: [124, 125],
  cards: [{ id: 124, name: 'Grizzly Bears' }, { id: 125, name: 'Lightning Bolt' }] };

// Land Grant's alternate cost reveals the hand before its search resolves.
test('proof refresh preserves the actual hand reveal over passive stack inspection', async () => {
  const passive = { ...hand, viewer: 1, inspector_only: true, description: 'Revealed while on the stack' };
  const current = { decision: { kind: 'priority', player: 1 }, viewed_cards: passive };
  const result = await preserveViewedCardsFromHint(current, { viewed_cards: hand });
  assert.deepEqual(result.viewed_cards, hand);
  assert.equal(result.decision, current.decision);
});

test('a later searched-card reveal replaces the earlier hand reveal', async () => {
  const forest = { ...hand, zone: 'library', card_ids: [130], cards: [{ id: 130, name: 'Forest' }] };
  const current = { viewed_cards: forest };
  assert.equal(await preserveViewedCardsFromHint(current, { viewed_cards: hand }), current);
  const different = { viewed_cards: { ...forest, inspector_only: true } };
  assert.equal(await preserveViewedCardsFromHint(different, { viewed_cards: hand }), different);
});

test('private or already passive hints never promote an inspection into a reveal', async () => {
  const current = { viewed_cards: { ...hand, inspector_only: true } };
  for (const hint of [{ ...hand, visibility: 'private' }, current.viewed_cards]) {
    assert.equal(await preserveViewedCardsFromHint(current, { viewed_cards: hint }), current);
  }
});
