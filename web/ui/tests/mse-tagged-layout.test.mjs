import test from 'node:test';
import assert from 'node:assert/strict';
import { layoutTaggedText } from '../src/lib/mse/tagged-layout.mjs';
import { parseTaggedText } from '../src/lib/mse/tagged-text.mjs';
import { MseImage } from '../src/lib/mse/script-values.mjs';
import { MseImageRenderer } from '../src/lib/mse/image-renderer.mjs';
import { parseAlignment } from '../src/lib/mse/alignment.mjs';

test('tagged layout remeasures mixed symbol and font runs at the selected scale', async () => {
  const source = '<b>AB</b><sym>W</sym>';
  const font = { name: 'Fixture', size: 20, scale_down_to: 5 };
  const fonts = { prepare: async () => ({ measure: (text,font,scale) => ({ width: text.length * 10 * scale, height: 40 * scale }), lineHeight: (font,scale) => 40 * scale }) };
  const symbols = {
    settings: { size: 20, scale_down_to: 5, alignment: 514 },
    font: { name: 'mana', horizontalSpace: 0, verticalSpace: 0, symbols: [{ code: 'W', enabled: true, drawText: -1, imageSize: 20, image: new MseImage('asset', { path: {} }) }] },
  };
  const renderer = new MseImageRenderer({ loadAsset: async () => ({ width: 20, height: 20, hasAlpha: true, data: new Uint8ClampedArray(1600).fill(255) }) });
  const result = await layoutTaggedText(parseTaggedText(source,font), fonts, symbols, renderer, { width: 100, height: 20, multiLine: false, alignment: parseAlignment('right middle') });
  assert.equal(result.fits, true);
  assert.equal(result.scale, 0.5);
  assert.equal(result.width, 20);
  assert.equal(result.height, 20);
  assert.equal(result.lines[0].positions[0], 80);
  assert.equal(result.measurement.characters.length, source.length);
  assert.equal(result.measurement.runs[1].measured.glyphs[0].bitmap.width, 10);
});

test('field stretch uses available width and overflow flags while preserving content metrics',async()=>{
  const font={name:'Fixture',size:10,scale_down_to:10};
  const fonts={prepare:async()=>({measure:text=>({width:text.length*10,height:10}),lineHeight:()=>10})};
  for(const [alignment,width,expected] of [['shrink-overflow',25,0.5],['shrink-overflow',65,1],['stretch',65,1.5],['left',25,1]]){
    const result=await layoutTaggedText(parseTaggedText('ABCD',font),fonts,null,null,{width,height:20,paddingLeft:2,paddingRight:3,multiLine:false,alignment:parseAlignment(alignment)});
    assert.equal(result.width,40);
    assert.equal(result.stretch,expected);
  }
});
