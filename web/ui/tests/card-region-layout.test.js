import test from 'node:test';
import assert from 'node:assert/strict';
import {registeredFieldFontSize, registeredFieldLayouts, registeredLinePitch, SCAN_ASPECT} from '../src/lib/card-region-layout.js';

// A face whose glyphs average half an em wide, whose ink spans 0.9em and whose
// content area (ascent plus descent) is 1.2em.
const measure = text => ({width: text.length * 50, height: 90, content: 120});
const line = (text, y, height = .03) => ({text, x: .12, y, width: text.length * .5 * 22 / 488, height});

test('registered type size follows whole-line widths, not per-line box heights', () => {
  const field = {kind: 'rule', lines: [line('Pay 1 life, Sacrifice another creature:', .65, .035), line('Put a counter on up to one target', .68, .029), line('creature and draw a card.', .71, .026)]};
  assert.ok(Math.abs(registeredFieldFontSize(field, measure) * 488 - 22) < 1e-9);
  assert.ok(Math.abs(registeredLinePitch(field) - .03) < 1e-9);
});

test('parenthetical reminder lines are measured in italics until the bracket closes', () => {
  const slanted = (text, italic) => ({width: text.length * (italic ? 45 : 50), height: 90, content: 120});
  const field = {kind: 'rule', lines: [
    line('Discard a card: Proliferate. (Choose any', .75), {...line('number of permanents, then give each', .78), width: 36 * .45 * 22 / 488},
    {...line('another counter.) Draw a card.', .81), width: 30 * .45 * 22 / 488}, line('Then discard a card at random.', .84)]};
  assert.ok(Math.abs(registeredFieldFontSize(field, slanted) * 488 - 22) < 1e-9, 'every line agrees once italics are applied');
});

test('symbol-heavy lines fall back to the box height against the face ink height', () => {
  const field = {kind: 'rule', lines: [{text: '©: Add ©.', x: .1, y: .6, width: .2, height: 22 * .9 / 680}]};
  assert.ok(Math.abs(registeredFieldFontSize(field, measure) * 488 - 22) < 1e-6);
  assert.equal(registeredFieldFontSize({kind: 'rule', lines: [{text: '©', x: .1, y: .6, width: .02, height: .03}]}, measure), null);
});

test('field layouts keep the printed baseline pitch and give single lines a full line box', () => {
  const rule = {kind: 'rule', lines: [line('Pay 1 life, Sacrifice another creature:', .65), line('Put a counter on up to one target', .68), line('creature and draw a card.', .71)]};
  rule.bounds = {x: .12, y: .65, width: .75, height: .09};
  const keyword = {kind: 'rule', lines: [line('Protection from Humans', .61, .02)], bounds: {x: .12, y: .61, width: .48, height: .02}};
  const unregistered = {kind: 'flavor', lines: []};
  const [multi, single, missing] = registeredFieldLayouts([rule, keyword, unregistered], () => measure);
  const size = 22 / 488;
  assert.ok(Math.abs(multi.lineHeight - .03 * SCAN_ASPECT / size) < 1e-9, 'pitch over size');
  assert.equal(single.lineHeight, multi.lineHeight, 'single lines borrow the paragraph pitch');
  assert.ok(Math.abs(multi.bounds.height - (2 * multi.lineHeight + 1.2) * size / SCAN_ASPECT) < 1e-9, 'two pitches plus the last content area');
  assert.ok(Math.abs(single.bounds.height - 1.2 * size / SCAN_ASPECT) < 1e-9, 'expanded to one content area');
  assert.ok(Math.abs((single.bounds.y + single.bounds.height / 2) - (.61 + .01)) < 1e-9, 'centred on the printed ink');
  assert.equal(missing, null);
});
