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

test('a flowed column keeps printed tops, closes gaps before shrinking, and flags displaced ink', async () => {
  const {registeredColumnFlow} = await import('../src/lib/card-region-layout.js');
  const items = [
    {index: 0, top: .61, footprint: .027, natural: .027},
    {index: 1, top: .653, footprint: .088, natural: .088},
    {index: 2, top: .756, footprint: .12, natural: .10},
  ];
  // Everything fits where it was printed: nothing moves, nothing shrinks.
  const still = registeredColumnFlow(items, {limit: .88, minGap: .01});
  assert.equal(still.displaced, false);
  assert.equal(still.shrink, 1);
  assert.deepEqual([...still.positions.values()].map(p => +p.top.toFixed(3)), [.61, .653, .756]);
  // A longer translation of the middle paragraph pushes the last one down into
  // the room below it instead of shrinking the type.
  const longer = registeredColumnFlow([items[0], {...items[1], natural: .1}, items[2]], {limit: .88, minGap: .01});
  assert.equal(longer.displaced, true);
  assert.equal(longer.shrink, 1);
  assert.ok(Math.abs(longer.positions.get(2).top - .763) < 1e-9);
  // When the column overruns the box even with the printed gaps closed to the
  // minimum, it asks for exactly the type scale that would fit.
  const crowded = registeredColumnFlow([items[0], {...items[1], natural: .13}, {...items[2], natural: .13}], {limit: .88, minGap: .01});
  assert.equal(crowded.displaced, true);
  assert.ok(crowded.shrink < 1 && crowded.shrink > .8, String(crowded.shrink));
  const bottom = crowded.positions.get(2).bottom;
  assert.ok(Math.abs((bottom - .61) * crowded.shrink - (.88 - .61)) < 1e-9);
  // Unmeasured paragraphs occupy their printed footprint and are never moved.
  const unknown = registeredColumnFlow(items.map(item => ({...item, natural: null})), {limit: .88, minGap: .01});
  assert.equal(unknown.displaced, false);
});

test('columns are built per face from measured heights that still describe the current text', async () => {
  const {registeredColumns} = await import('../src/lib/card-region-layout.js');
  const fields = [
    {kind: 'rule', face: 0, bounds: {x: .1, y: .61, width: .7, height: .026}},
    {kind: 'rule', face: 0, bounds: {x: .1, y: .653, width: .7, height: .088}},
    {kind: 'stats', face: 0, bounds: {x: .83, y: .9, width: .09, height: .035}},
  ];
  const layouts = [
    {size: .04, lineHeight: 1, span: .03, bounds: {x: .1, y: .608, width: .7, height: .03}},
    {size: .04, lineHeight: 1, span: .092, bounds: {x: .1, y: .651, width: .7, height: .2}},
    {size: .04, lineHeight: 1, span: .035, bounds: {x: .83, y: .9, width: .09, height: .035}},
  ];
  const unit = 488, height = unit * SCAN_ASPECT;
  const measured = new Map([[0, {px: .08 * height, unit, scale: 1, text: 'a'}], [1, {px: .05 * height, unit, scale: .9, text: 'b'}]]);
  const columns = registeredColumns(fields, layouts, ['a', 'b', '2/4'], measured, {unit, scale: 1});
  // The first paragraph tripled in height and pushes the second down; the
  // second's stale report (another scale) counts as its printed footprint.
  assert.ok(columns.forced.has(0));
  assert.ok(columns.positions.get(1).top > .651);
  assert.equal(columns.positions.get(1).limit, .869);
  assert.equal(columns.shrink, 1);
  assert.equal(registeredColumns(fields, layouts, ['a', 'b', '2/4'], measured, {unit: 0, scale: 1}), null);
});

test('level bands and side boxes are not flowed as one column', async () => {
  const {registeredColumns} = await import('../src/lib/card-region-layout.js');
  const fields = [
    {kind: 'rule', face: 0, bounds: {x: .1, y: .62, width: .7, height: .05}},
    {kind: 'rule', face: 0, bounds: {x: .1, y: .7, width: .15, height: .02}},
    {kind: 'rule', face: 0, bounds: {x: .8, y: .7, width: .08, height: .03}},
  ];
  const layouts = fields.map(f => ({size: .04, lineHeight: 1, span: f.bounds.height, bounds: {...f.bounds}}));
  const unit = 488, height = unit * SCAN_ASPECT;
  const measured = new Map(fields.map((f, i) => [i, {px: .2 * height, unit, scale: 1, text: String(i)}]));
  const columns = registeredColumns(fields, layouts, ['0', '1', '2'], measured, {unit, scale: 1});
  assert.equal(columns.positions.size, 0);
  assert.equal(columns.forced.size, 0);
  assert.equal(columns.shrink, 1);
});

test('a generic mana digit run into the name line is trimmed off the name box', async () => {
  const {trimRegisteredNameCosts} = await import('../src/lib/card-region-layout.js');
  const measureName = text => ({width: text.length * 50});
  const merged = {kind: 'name', text: 'Yawgmoth, Thran Physician', lines: [{text: 'Yawgmoth, Thran Physician 2', x: .1, y: .05, width: .54, height: .05}], bounds: {x: .1, y: .05, width: .54, height: .05}};
  const [trimmed] = trimRegisteredNameCosts([merged], measureName);
  assert.equal(trimmed.lines[0].text, 'Yawgmoth, Thran Physician');
  // One printed pip: a disc .7 line heights wide after a quarter-line gap.
  const disc = .05 * SCAN_ASPECT, expected = .54 - disc * .7 - disc * .25;
  assert.ok(expected < .54 * 25 / 27, 'the disc estimate is the tighter one here');
  assert.ok(Math.abs(trimmed.lines[0].width - expected) < 1e-9);
  assert.ok(Math.abs(trimmed.bounds.width - expected) < 1e-9);
  assert.ok(Math.abs(trimmed.limit - (.1 + expected + disc * .2)) < 1e-9, 'translations stop where the cost begins');
  // Spelling variants, casing and translations are left alone.
  const [variant] = trimRegisteredNameCosts([{...merged, lines: [{...merged.lines[0], text: 'Tarmogoyi'}], text: 'Tarmogoyf'}], measureName);
  assert.equal(variant.lines[0].text, 'Tarmogoyi');
  const [upper] = trimRegisteredNameCosts([{...merged, lines: [{...merged.lines[0], text: 'TREASURE'}], text: 'Treasure'}], measureName);
  assert.equal(upper.lines[0].width, .54);
});

test('line boxes split by a mana symbol merge back into one printed line', async () => {
  const {mergeRegisteredLineSegments} = await import('../src/lib/card-region-layout.js');
  const split = {kind: 'rule', lines: [
    {text: '({T}: Add', x: .31, y: .735, width: .205, height: .035},
    {text: 'or {R}.)', x: .553, y: .738, width: .139, height: .032},
  ]};
  const [merged] = mergeRegisteredLineSegments([split]);
  assert.equal(merged.lines.length, 1, 'one printed line');
  assert.equal(merged.lines[0].text, '({T}: Add or {R}.)');
  assert.ok(Math.abs(merged.lines[0].x - .31) < 1e-9);
  // The span covers the gap the symbols sit in, so the mask reaches them.
  assert.ok(Math.abs(merged.lines[0].width - (.553 + .139 - .31)) < 1e-9);
  assert.ok(Math.abs(merged.lines[0].height - (.738 + .032 - .735)) < 1e-9);
  // Successive printed lines of a paragraph are not on one baseline.
  const paragraph = {kind: 'rule', lines: [
    {text: 'first line', x: .12, y: .65, width: .7, height: .03},
    {text: 'second line', x: .12, y: .69, width: .6, height: .03},
  ]};
  assert.equal(mergeRegisteredLineSegments([paragraph])[0].lines.length, 2);
  assert.equal(mergeRegisteredLineSegments([{kind: 'name', lines: [paragraph.lines[0]]}])[0].lines.length, 1);
});

test('a line the printing centres on its text box is centred over the whole column', () => {
  const type = {kind: 'type', bounds: {x: .078, y: .58, width: .5, height: .03}, lines: [line('Land — Island Mountain', .58, .03)]};
  const centred = {kind: 'rule', bounds: {x: .307, y: .735, width: .385, height: .035}, lines: [line('({T}: Add {U} or {R}.)', .735, .035)]};
  const [, laid] = registeredFieldLayouts([type, centred], () => measure);
  assert.equal(laid.centred, true);
  assert.ok(Math.abs(laid.bounds.x - .078) < 1e-9, 'starts at the column inset');
  assert.ok(Math.abs(laid.bounds.width - (1 - .078 * 2)) < 1e-9, 'spans the mirrored column');
  // A line at the column inset is ordinary left-aligned text.
  const ordinary = {kind: 'rule', bounds: {x: .085, y: .735, width: .7, height: .035}, lines: [line('Ordinary rules text here.', .735, .035)]};
  const [, plain] = registeredFieldLayouts([type, ordinary], () => measure);
  assert.equal(plain.centred, false);
  assert.ok(Math.abs(plain.bounds.x - .085) < 1e-9);
  // A box off to one side (level bands, side panels) is not centred either.
  const aside = {kind: 'rule', bounds: {x: .8, y: .735, width: .1, height: .035}, lines: [line('4/4', .735, .035)]};
  assert.equal(registeredFieldLayouts([type, aside], () => measure)[1].centred, false);
});
