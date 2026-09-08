import test from 'node:test';
import assert from 'node:assert/strict';
import {registeredFieldFontSize, registeredFieldLayouts, registeredLinePitch, registrationGeometryIsUsable, SCAN_ASPECT} from '../src/lib/card-region-layout.js';

test('unusable OCR geometry preserves the scan instead of replacing text across other fields',()=>{
  const name={kind:'name',bounds:{x:.1,y:.05,width:.5,height:.04}};
  const rule={kind:'rule',bounds:{x:.1,y:.6,width:.7,height:.25}};
  assert.equal(registrationGeometryIsUsable({fields:[name,rule]}),true);
  assert.equal(registrationGeometryIsUsable({fields:[name,{...rule,bounds:{...rule.bounds,y:.05,height:.8}}]}),false);
  assert.equal(registrationGeometryIsUsable({fields:[{...name,bounds:{x:.1,y:.3,width:.04,height:.5}},rule]}),false);
  assert.equal(registrationGeometryIsUsable({fields:[name,{...rule,unprinted:true,bounds:name.bounds}]}),true);
});

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
  assert.ok(multi.bounds.height >= (2 * multi.lineHeight + 1.2) * size / SCAN_ASPECT - 1e-9, 'at least two pitches plus the last content area');
  assert.ok(Math.abs(multi.bounds.y + multi.bounds.height - (.875 - .006)) < 1e-9, 'the last paragraph runs to the foot of the text box');
  assert.ok(Math.abs(single.bounds.height - 1.2 * size / SCAN_ASPECT) < 1e-9, 'expanded to one content area');
  assert.ok(Math.abs((single.bounds.y + single.bounds.height / 2) - (.61 + .01)) < 1e-9, 'centred on the printed ink');
  assert.equal(missing, null);
});

test('fields grow into the room the frame has for longer translations', () => {
  const name = {kind: 'name', limit: .7, lines: [line('Braids, Arisen Nightmare', .05, .04)], bounds: {x: .1, y: .05, width: .4, height: .04}};
  const type = {kind: 'type', lines: [line('Legendary Creature', .57, .03)], bounds: {x: .1, y: .57, width: .4, height: .03}};
  const first = {kind: 'rule', lines: [line('First ability text here.', .62, .025)], bounds: {x: .1, y: .62, width: .7, height: .025}};
  const last = {kind: 'rule', lines: [line('Second ability text here.', .66, .025)], bounds: {x: .1, y: .66, width: .7, height: .025}};
  const flavor = {kind: 'flavor', lines: [line('Some italic flavor line.', .8, .025)], bounds: {x: .1, y: .8, width: .6, height: .025}};
  const [n, t, f, l, fl] = registeredFieldLayouts([name, type, first, last, flavor], () => measure);
  assert.ok(Math.abs(n.bounds.width - (.7 - .012 - .1)) < 1e-9, 'name stops before the mana cost');
  assert.ok(Math.abs(t.bounds.width - (.84 - .1)) < 1e-9, 'type stops before the set symbol');
  assert.ok(f.bounds.height < .04, 'earlier paragraphs keep their printed room');
  const keyword = {kind: 'rule', lines: [line('Flying', .60, .025)], bounds: {x: .1, y: .60, width: .1, height: .025}};
  const [k] = registeredFieldLayouts([keyword, first], () => measure);
  assert.ok(Math.abs(k.bounds.width - .7) < 1e-9, 'keyword lines span the paragraph column');
  assert.ok(Math.abs(l.bounds.y + l.bounds.height - (.8 - .006)) < 1e-9, 'last paragraph runs down to the flavor text');
  assert.ok(Math.abs(fl.bounds.width - .6) < 1e-9);
});

test('registrations match the pinned scan by path, or another language of the same printing by set and number', async () => {
  const {registrationForImage, registrationForPrinting} = await import('../src/lib/card-region-layout.js');
  const front = {id: 'a', set: 'dmr', collector_number: '386', source: 'https://cards.scryfall.io/normal/front/0/5/051386e0-4c1c-48ed-8883-664c719cf0fe.jpg?1787729781'};
  const back = {id: 'b', set: 'mid', collector_number: '4', face: 1, source: 'https://cards.scryfall.io/normal/back/1/2/12345678-1234-1234-1234-123456789abc.jpg?1'};
  const catalog = [front, back];
  assert.equal(registrationForImage(catalog, 'https://cards.scryfall.io/normal/front/0/5/051386e0-4c1c-48ed-8883-664c719cf0fe.jpg?9'), front, 'CDN revision does not matter');
  assert.equal(registrationForImage(catalog, 'https://cards.scryfall.io/normal/front/8/0/80dee03f-ddb9-476c-b880-30a9abec688f.jpg'), null, 'a translated scan has its own id');
  const spanish = {set: 'dmr', collector_number: '386', lang: 'es'};
  assert.equal(registrationForPrinting(catalog, spanish, 'https://cards.scryfall.io/normal/front/8/0/80dee03f-ddb9-476c-b880-30a9abec688f.jpg'), front);
  assert.equal(registrationForPrinting(catalog, {set: 'DMR', collector_number: '386'}), front, 'set codes compare case-insensitively');
  assert.equal(registrationForPrinting(catalog, {set: 'mid', collector_number: '4'}, 'https://cards.scryfall.io/normal/back/9/9/99999999-1234-1234-1234-123456789abc.jpg'), back, 'faces follow the scan side');
  assert.equal(registrationForPrinting(catalog, {set: 'mid', collector_number: '4'}, 'https://cards.scryfall.io/normal/front/9/9/99999999-1234-1234-1234-123456789abc.jpg'), null, 'the unregistered front face stays unregistered');
  assert.equal(registrationForPrinting(catalog, {set: 'dmr', collector_number: '387'}), null);
  assert.equal(registrationForPrinting(catalog, null), null);
});
