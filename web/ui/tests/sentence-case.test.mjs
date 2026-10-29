import test from 'node:test';
import assert from 'node:assert/strict';
import {capitalizeSentences} from '../src/lib/sentence-case.js';

test('capitalizes the first letter after a period, across spaces and line breaks', () => {
  assert.equal(capitalizeSentences('Flying. draw a card.\nthen discard a card.'), 'Flying. Draw a card.\nThen discard a card.');
  assert.equal(capitalizeSentences('Tap it. (it doesn\'t untap.) you lose 2 life.'), 'Tap it. (It doesn\'t untap.) You lose 2 life.');
  assert.equal(capitalizeSentences('Exile it. "whenever you cast"'), 'Exile it. "Whenever you cast"');
  assert.equal(capitalizeSentences('Robas una carta. él pierde.'), 'Robas una carta. Él pierde.');
});

test('leaves decimals, URLs and sentence starts without a period alone', () => {
  for (const text of ['Costs {.5} less', 'see chiplis.com/help', 'flying\nvigilance', '', null])
    assert.equal(capitalizeSentences(text), text);
});
