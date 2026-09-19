import test from 'node:test';
import assert from 'node:assert/strict';
import {cardFrameTone, cardFrameColors, explicitCardColors, frameToneForColors, cardFrameToneStyle} from '../src/lib/card-frame-tone.js';

test('live object colors win over the printing and the mana cost', () => {
  const devoid = {characteristic_signature: 'name:Probe\ncolors:-\n', mana_cost: '{2}{U}'};
  assert.deepEqual(explicitCardColors(devoid), []);
  assert.deepEqual(cardFrameColors({cards: [devoid], printing: {colors: ['U']}, manaCost: '{2}{U}'}), []);
  const painted = {characteristic_signature: 'colors:Red,Green'};
  assert.deepEqual(cardFrameColors({cards: [null, painted], manaCost: '{W}'}), ['R', 'G']);
});

test('objects without color information fall through to the printing, then the cost', () => {
  assert.equal(explicitCardColors({name: 'No colors here'}), null);
  assert.deepEqual(cardFrameColors({cards: [{name: 'x'}], printing: {colors: ['B']}}), ['B']);
  assert.deepEqual(cardFrameColors({cards: [{name: 'x'}], printing: {}, manaCost: '{W/U}{1}'}), ['W', 'U']);
  assert.deepEqual(cardFrameColors({}), []);
});

test('tones follow the color count, then the type line', () => {
  assert.equal(frameToneForColors(['R']), 'red');
  assert.equal(frameToneForColors(['U', 'R']), 'gold');
  assert.equal(frameToneForColors(['W', 'U', 'B']), 'gold');
  assert.equal(frameToneForColors([], 'Basic Land — Forest'), 'land');
  assert.equal(frameToneForColors([], 'Artifact Creature — Golem'), 'artifact');
  assert.equal(frameToneForColors([], 'Creature — Eldrazi'), 'colorless');
  assert.equal(frameToneForColors(['G'], 'Land Creature — Forest Dryad'), 'green');
});

test('the style carries the palette, split between exactly two colors', () => {
  const single = cardFrameToneStyle('blue', ['U']);
  assert.equal(single['--card-frame-edge'], '#467995');
  assert.equal(single['--card-frame-edge-fill'], undefined);
  const dual = cardFrameToneStyle('gold', ['U', 'R']);
  assert.equal(dual['--card-frame-bar'], '#b89b55');
  assert.match(dual['--card-frame-edge-fill'], /^linear-gradient\(90deg, #467995 .* #8b473b 100%\)$/);
  const triple = cardFrameToneStyle('gold', ['W', 'U', 'B']);
  assert.equal(triple['--card-frame-edge-fill'], undefined);
  assert.equal(triple['--card-frame-edge'], '#987c41');
});

test('cardFrameTone resolves a token from its printing colors', () => {
  const monk = cardFrameTone({cards: [{name: 'Monk'}], printing: {colors: ['R'], layout: 'token'}, manaCost: '', typeLine: 'Token Creature — Monk'});
  assert.equal(monk.tone, 'red');
  assert.deepEqual(monk.colors, ['R']);
  assert.equal(monk.style['--card-frame-accent'], '#9b493b');
});
