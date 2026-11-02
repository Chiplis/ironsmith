import test from 'node:test';
import assert from 'node:assert/strict';
import {bindLinkedEntryExileReference as bind} from '../src/lib/linked-entry-exile-references.js';
const definition = {linkedExileVerb: 'consumed'};
test('historical entry predicates retain comma modifiers and the complete scalar filter', () => {
  assert.equal(bind('If it consumed a noncreature, nonland card, draw a card.', definition),
    'If a noncreature, nonland card was exiled as this creature entered, draw a card.');
  assert.equal(bind('It gets +1/+1 if it consumed a creature card with power 3 or greater.', definition),
    'It gets +1/+1 if a creature card with power 3 or greater was exiled as this creature entered.');
  assert.equal(bind('If this permanent consumed an artifact card with mana value 2 or less, you gain 1 life.', definition),
    'If an artifact card with mana value 2 or less was exiled as this creature entered, you gain 1 life.');
});
test('the explicit source name and verb are data; a sibling instruction is separate', () => {
  assert.equal(bind('If Sample, the Other consumed a creature card and you gained life, draw a card.', definition, 'Sample, the Other'),
    'If a creature card was exiled as this creature entered and you gained life, draw a card.');
  assert.equal(bind('If it consumed a creature card and exiled a land card, draw a card.', definition),
    'If a creature card was exiled as this creature entered and exiled a land card, draw a card.');
});
test('quoted programs, absent declarations, and unrecognized trailing qualifiers remain opaque', () => {
  const quoted = 'This permanent has "If it consumed a creature card, draw a card."';
  assert.equal(bind(quoted, definition), quoted);
  const text = 'If it consumed a creature card with flying, draw a card.';
  assert.equal(bind(text, definition), text);
  assert.equal(bind('If it consumed a creature card, draw a card.', {}), 'If it consumed a creature card, draw a card.');
  assert.equal(bind('If it ate a creature card, draw a card.', definition), 'If it ate a creature card, draw a card.');
});
