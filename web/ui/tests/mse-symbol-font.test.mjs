import test from 'node:test';
import assert from 'node:assert/strict';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';
import { loadSymbolFont, evaluateSymbolFont, splitSymbols } from '../src/lib/mse/symbol-font.mjs';

const document = `image font size: 100
horizontal space: 2
symbol:
\tcode: W
\tenabled: {alternate}
\timage: {recolor_image("alt.png", color: "white")}
symbol:
\tcode: W
\timage: white.png
symbol:
\tcode: W/U
\timage: hybrid.png
symbol:
\tcode: ([0-9]+)
\tregex: true
\tdraw text: 1
\timage: number.png
\ttext font:
\t\tname: MPlantin
\t\tsize: {font_size}
`;
test('symbol fonts preserve declaration precedence, regex captures and package ownership', async () => {
  const store = new MsePackageStore({ readText: async () => document });
  const definition = await loadSymbolFont(store, 'mana');
  assert.equal(await loadSymbolFont(store, 'mana.mse-symbol-font'), definition);
  const runtime = new MseRuntime({ builtins: coreBuiltins(), variables: { alternate: false, font_size: 12 } });
  const font = evaluateSymbolFont(definition, runtime);
  const symbols = splitSymbols(font, 'W/U12?W');
  assert.deepEqual(symbols.map(s => [s.text, s.drawText, s.offset]), [['W','',0], ['12','12',3], ['W','',6]]);
  assert.equal(symbols[0].symbol.image.parameters.path.key, 'mana.mse-symbol-font/white.png');
  assert.equal(symbols[1].symbol.font.size, 12);
  assert.equal(symbols[1].symbol.imageSize, 100);
  runtime.scope.set('alternate', true);
  const updated = splitSymbols(evaluateSymbolFont(definition, runtime), 'W');
  assert.equal(updated[0].symbol.image.operation, 'recolor');
  assert.equal(updated[0].symbol.image.parameters.image.parameters.path.package, 'mana.mse-symbol-font');
  assert.equal(font.symbols[0].enabled, false);
});

test('zero length regexes and unknown Unicode characters cannot stall symbol splitting', async () => {
  const store = new MsePackageStore({ readText: async () => 'symbol:\n\tcode: a*\n\tregex: true\n\timage: a.png\n' });
  const font = evaluateSymbolFont(await loadSymbolFont(store, 'empty'), new MseRuntime());
  assert.deepEqual(splitSymbols(font, '😀aa?').map(s => [s.text, s.offset]), [['aa',2]]);
});

import { measureSymbolRun } from '../src/lib/mse/symbol-layout.mjs';
import { MseImageRenderer } from '../src/lib/mse/image-renderer.mjs';

test('symbol measurement keeps fractional advances while bitmap sizes truncate', async () => {
  const store = new MsePackageStore({ readText: async () => document });
  const runtime = new MseRuntime({ builtins: coreBuiltins(), variables: { alternate: false, font_size: 12 } });
  const font = evaluateSymbolFont(await loadSymbolFont(store, 'mana'), runtime);
  const run = { font, settings: { size: 15, alignment: 514 }, symbols: splitSymbols(font, '12WW') };
  let decoded = 0;
  const renderer = new MseImageRenderer({ loadAsset: async () => {
    decoded++;
    return { width: 101, height: 99, hasAlpha: false, data: new Uint8ClampedArray(101 * 99 * 4).fill(255) };
  } });
  const result = await measureSymbolRun(run, renderer);
  assert.equal(decoded, 2);
  assert.ok(Math.abs(result.width - 51.45) < 1e-12);
  assert.ok(Math.abs(result.height - 15.85) < 1e-12);
  assert.deepEqual(result.glyphs.map(g => [g.bitmap.width, g.bitmap.height]), [[15,14],[15,14],[15,14]]);
  assert.ok(Math.abs(result.glyphs[0].x - 1.075) < 1e-12);
  assert.deepEqual(result.characters.map(c => c.break), ['no','maybe','maybe','maybe']);
  assert.ok(Math.abs(result.characters.reduce((a,c) => a+c.width, 0) - result.width) < 1e-12);
  const tiny = await measureSymbolRun(run, renderer, { scale: 0.001 });
  assert.deepEqual([...tiny.glyphs[0].bitmap.data], [0,0,0,0]);
  assert.ok(tiny.width < 1);
});
