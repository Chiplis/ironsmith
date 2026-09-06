import test from 'node:test';
import assert from 'node:assert/strict';
import { fullCardImageUrl, materialColor, sectionInk, artFrameRails, artBottomRail, reconstructPanel, classifyTitlePanel, classifyTypePanel, printedGlyphHeight, detectPanelBounds } from '../src/lib/card-frame-colors.js';

test('color reference preserves the displayed printing, face, and cache version', () => {
  assert.equal(fullCardImageUrl('https://cards.scryfall.io/art_crop/back/a/b/id.jpg?123'), 'https://cards.scryfall.io/normal/back/a/b/id.jpg?123');
  assert.equal(fullCardImageUrl('https://custom.example/art.jpg'), '');
});

test('printed glyphs do not replace the dominant panel material', () => {
  const data = new Uint8ClampedArray(400 * 4);
  for (let i = 0; i < 400; i++) data.set(i < 150 ? [5, 5, 5, 255] : [230, 218, 190, 255], i * 4);
  assert.deepEqual(materialColor(data), [230, 218, 190]);
});

test('ink sampling handles light and dark printed lettering', () => {
  for (const [paper, ink] of [[[40, 45, 55], [240, 235, 220]], [[235, 230, 205], [15, 20, 25]]]) {
    const width = 100, height = 30, data = new Uint8ClampedArray(width * height * 4);
    for (let i = 0; i < width * height; i++) data.set([...paper, 255], i * 4);
    for (const left of [20, 40, 60]) for (let y = 8; y < 22; y++) for (let x = left; x < left + 4; x++) data.set([...ink, 255], (y * width + x) * 4);
    assert.deepEqual(sectionInk({width, height, data}), ink);
  }
});


test('art-border continuation requires two supported edges and leaves its center transparent', () => {
  const width = 200, height = 160;
  const make = (left, right) => {
    const data = new Uint8ClampedArray(width * height * 4);
    for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
      const border = (left && x < 10) || (right && x >= width - 10);
      data.set(border ? [70, 45, 30, 255] : [180, 210, 240, 255], (y * width + x) * 4);
    }
    return {data,width,height};
  };
  assert.equal(artFrameRails(make(false, false)), null);
  assert.equal(artFrameRails(make(true, false)), null);
  const rails = artFrameRails(make(true, true));
  assert.ok(rails);
  assert.ok(rails.left > .03 && rails.left < .08);
  assert.equal(rails.strip[3], 255);
  assert.equal(rails.strip[100 * 4 + 3], 0);
});


test('panel reconstruction removes light and dark print while preserving clean texture', () => {
  for (const [paper, ink] of [[65, 245], [225, 15]]) {
    const width = 180, height = 40, data = new Uint8ClampedArray(width * height * 4);
    for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
      const grain = (x * 7 + y * 3) % 9;
      const value = x < 110 && x % 18 < 4 && y > 12 && y < 27 ? ink : paper + grain;
      data.set([value, value, value, 255], (y * width + x) * 4);
    }
    const panel = reconstructPanel({ data, width, height });
    assert.ok(panel);
    assert.equal(panel.width, width);
    assert.equal(panel.height, height);
    for (let p = 0; p < width * height; p++) {
      assert.ok(panel.data[p * 4] >= paper && panel.data[p * 4] <= paper + 8, 'printed ink is removed');
      if (!panel.mask[p]) assert.equal(panel.data[p * 4], data[p * 4], 'original clean texture is unchanged');
    }
    assert.deepEqual(reconstructPanel({data, width, height}).data, panel.data, 'stable between renders');
  }
});


test('bottom rail detection requires a boundary across most of the art', () => {
  const width = 240, height = 180;
  const make = framed => {
    const data = new Uint8ClampedArray(width * height * 4);
    for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
      const value = y >= height - 6 && (framed || x < width / 3) ? 50 : 180;
      data.set([value, value, value, 255], (y * width + x) * 4);
    }
    return {data, width, height};
  };
  assert.equal(artBottomRail(make(false)), 0);
  assert.ok(artBottomRail(make(true)) >= 5);
});


test('title classifier requires enclosing strokes on both sides, regardless of material color', () => {
  const width = 488, height = 680;
  for (const paper of [65, 205]) {
    const make = sides => {
      const data = new Uint8ClampedArray(width * height * 4);
      for (let p = 0; p < width * height; p++) data.set([paper, paper, paper, 255], p * 4);
      const ink = paper < 100 ? 245 : 20;
      for (const x of sides) for (let y = 35; y < 65; y++) data.set([ink, ink, ink, 255], (y * width + x) * 4);
      return {data, width, height};
    };
    assert.equal(classifyTitlePanel(make([])).kind, 'integrated');
    assert.equal(classifyTitlePanel(make([32])).kind, 'integrated');
    const result = classifyTitlePanel(make([32, width - 33]));
    assert.equal(result.kind, 'panel');
    assert.deepEqual(result.border, Array(3).fill(paper < 100 ? 245 : 20));
  }
});


test('type enclosure is classified independently of the title', () => {
  const width = 488, height = 680, data = new Uint8ClampedArray(width * height * 4);
  for (let p = 0; p < width * height; p++) data.set([210, 210, 210, 255], p * 4);
  for (const x of [32, width - 33]) for (let y = 382; y < 417; y++) data.set([20, 20, 20, 255], (y * width + x) * 4);
  const scan = {data, width, height};
  assert.equal(classifyTitlePanel(scan).kind, 'integrated');
  assert.equal(classifyTypePanel(scan).kind, 'panel');
  assert.deepEqual(classifyTypePanel(scan).border, [20, 20, 20]);
});


test('printed glyph sizing uses repeated letters and ignores borders and punctuation', () => {
  for (const [paper, ink] of [[230, 20], [35, 240]]) {
    const width = 160, height = 40, data = new Uint8ClampedArray(width * height * 4);
    for (let p = 0; p < width * height; p++) data.set([paper, paper, paper, 255], p * 4);
    const mark = (x, y) => data.set([ink, ink, ink, 255], (y * width + x) * 4);
    for (let x = 0; x < width; x++) mark(x, 1);
    assert.equal(printedGlyphHeight({data, width, height}), null);
    for (const left of [12, 32, 52, 72, 92, 112]) {
      for (let y = 12; y < 24; y++) for (let x = left; x < left + 4; x++) mark(x, y);
      mark(left + 7, 28);
    }
    assert.equal(printedGlyphHeight({data, width, height}), 12);
  }
});


test('whole-panel bounds reject blank scans and locate an enclosed title', () => {
  const width = 488, height = 680, data = new Uint8ClampedArray(width * height * 4);
  for (let p = 0; p < width * height; p++) data.set([65,65,65,255],p*4);
  assert.equal(detectPanelBounds({data,width,height}, 'title'), null);
  for (let y = 32; y <= 72; y++) for (let x = 32; x <= 455; x++) data.set([220,220,220,255],(y*width+x)*4);
  const bounds = detectPanelBounds({data,width,height}, 'title');
  assert.ok(bounds);
  assert.ok(Math.abs(bounds.x-32) <= 4);
  assert.ok(Math.abs(bounds.y-32) <= 4);
  assert.ok(bounds.width > 415 && bounds.width < 435);
  assert.equal(detectPanelBounds({data,width,height}, 'rules'), null);
});
