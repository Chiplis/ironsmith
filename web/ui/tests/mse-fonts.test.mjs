import test from 'node:test';
import assert from 'node:assert/strict';
import { resolveFontFace } from '../src/lib/mse/font-definition.mjs';

test('font face resolution honors italic faces, tagged overrides and Bold suffixes', () => {
  const font = { name: 'Beleren Bold', italic_name: 'Beleren Italic', bold: false, italic: false };
  assert.deepEqual(resolveFontFace(font), { name: 'Beleren', bold: true, italic: false });
  assert.deepEqual(resolveFontFace(font, { italic: true }), { name: 'Beleren Italic', bold: false, italic: false });
  assert.deepEqual(resolveFontFace(font, { italic: true, family: 'Matrix', fromTag: true }), { name: 'Matrix', bold: false, italic: true });
});

import { parseAlignment, alignmentOffset } from '../src/lib/mse/alignment.mjs';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { loadMseTemplate, MseTemplateContext } from '../src/lib/mse/template.mjs';

test('alignment parsing uses native precedence and fill modifiers', () => {
  assert.equal(parseAlignment('right center left bottom middle top'), 8192 | 4 | 1024);
  assert.equal(parseAlignment('center middle shrink-overflow force'), 2 | 512 | 16 | 4096);
  assert.equal(parseAlignment('justify-all'), 257 | 64 | 8192);
  assert.deepEqual(alignmentOffset(parseAlignment('middle right'), 100, 40, 30, 10), { x: 70, y: 15 });
});

test('symbol font defaults and scripted settings are distinct from ordinary fonts', async () => {
  const files = {
    'example.mse-game/game': 'card field:\n\tname: cost\n\ttype: text\n',
    'example.mse-style/style': `game: example
card width: 100
card height: 100
styling field:
\tname: large
\ttype: boolean
card style:
\tcost:
\t\tfont:
\t\t\tname: MPlantin
\t\tsymbol font:
\t\t\tname: {if styling.large then "magic-mana-large" else "magic-mana-small"}
\t\t\tsize: {if styling.large then 16 else 12}
\t\t\talignment: {if styling.large then "middle right" else "top left"}
`,
  };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => files[path.key] }), 'example.mse-style');
  const ctx = new MseTemplateContext(template).initialize();
  assert.equal(ctx.font('cost').name, 'MPlantin');
  const font = ctx.font('cost', 'card', true);
  assert.equal(font.name, 'magic-mana-large');
  assert.equal(font.size, 16);
  assert.equal(font.scale_down_to, 1);
  assert.equal(font.alignment, 8192 | 512 | 4);
  ctx.setField('styling', 'large', false);
  assert.equal(ctx.font('cost', 'card', true).name, 'magic-mana-small');
  assert.equal(ctx.layout().fields[0].properties.symbol_font.size, 12);
});
