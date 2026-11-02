import test from 'node:test';
import assert from 'node:assert/strict';
import { expandStatusEventReferences } from '../../web/ui/src/lib/status-event-references.js';

test('passive designation actions in trigger heads retain event meaning', () => {
  assert.equal(expandStatusEventReferences('When this Rift enters or is traversed, draw a card.', 'traversed'),
    'When this Rift enters or gains the traversed status, draw a card.');
  assert.equal(expandStatusEventReferences('Whenever creatures are moon blessed, draw a card.', 'moon blessed'),
    'Whenever creatures gain the moon blessed status, draw a card.');
});
test('state clauses and intervening conditions stay predicates', () => {
  for (const text of [
    'As long as this permanent is traversed, it gets +1/+1.',
    'When this creature attacks, if it is traversed, draw a card.',
    'If this permanent is traversed, draw a card.',
    'This permanent is traversed.',
    'Whenever this creature is untapped, if it is traversed, draw a card.',
  ]) assert.equal(expandStatusEventReferences(text, 'traversed'), text);
});
