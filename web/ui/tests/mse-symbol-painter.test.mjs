import test from 'node:test';
import assert from 'node:assert/strict';
import { prepareSymbolDrawing } from '../src/lib/mse/symbol-painter.mjs';

test('symbol drawing uses line height, bitmap margins, and fits multi-digit overlays', async () => {
  const calls = [], bitmap = { width: 18, height: 19 };
  const font = { size: 1, max_stretch: 0.8 };
  const measured = { settings: { alignment: 514 }, fontSize: 15, glyphs: [
    { text: '12', drawText: '12', bitmap, symbol: { font, alignment: 514, margins: { left: 0.1, right: 0.1, top: 0, bottom: 0 } } },
  ] };
  const fonts = { prepare: async () => ({
    measure: (text, font, scale) => ({ width: 2 * scale, height: scale }),
    draw: (ctx, text, x, y, scale) => calls.push(['text', text, scale]),
  }) };
  const draw = await prepareSymbolDrawing(measured, fonts, { createBitmap: () => 'bitmap' });
  const ctx = { drawImage: (...args) => calls.push(args), save() {}, restore() {}, translate: (...args) => calls.push(args), scale: (...args) => calls.push(args) };
  draw(ctx, { start: 3, end: 5, positions: [10,20,30], top: 5, height: 30 }, 3);
  assert.deepEqual(calls[0], ['bitmap',11,10.5]);
  assert.deepEqual(calls[1], [12.5,15.375]);
  assert.deepEqual(calls[2], [15/18.5,1]);
  assert.deepEqual(calls[3], ['text','12',9.25]);
  calls.length = 0;
  draw(ctx, { start: 5, end: 7, positions: [10,20,30], top: 5, height: 30 }, 3);
  assert.deepEqual(calls, []);
});

test('symbol shadows draw behind all glyphs and scale their displacement', async () => {
  const source={width:1,height:1,hasAlpha:true,data:new Uint8ClampedArray([0,0,0,255])};
  const glyph={text:'W',drawText:'',bitmap:source,symbol:{}};
  const measured={settings:{alignment:257,shadow_color:{channels:[1,2,3,255]},shadow_displacement_x:2},fontSize:30,glyphs:[glyph,glyph]};
  const calls=[];
  const draw=await prepareSymbolDrawing(measured,{}, {createBitmap:b=>b===source?'glyph':'shadow'});
  draw({drawImage:(...args)=>calls.push(args)}, {start:0,end:2,positions:[0,2,4],top:0,height:1},0);
  assert.deepEqual(calls,[['shadow',4,0],['shadow',6,0],['glyph',0,0],['glyph',2,0]]);
});
