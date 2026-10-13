import test from 'node:test';
import assert from 'node:assert/strict';
import { parseTaggedText, measureTaggedText } from '../src/lib/mse/tagged-text.mjs';
import { color } from '../src/lib/mse/color-values.mjs';
const font = { name: 'Base', italic_name: 'Italic', size: 12, scale_down_to: 8, color: color('black'), separator_color: color('gray'), shadow_displacement_x: 2, shadow_displacement_y: 3 };

test('nested formatting and scoped font overrides restore after closing tags', () => {
  const result = parseTaggedText('A<b>B<i>C</i>D</b><font:Other><size:18>E</size>F</font>G\u0001', font);
  assert.deepEqual(result.runs.map(r => r.text), ['A','B','C','D','E','F','G<']);
  assert.equal(result.runs[2].font.bold, true);
  assert.equal(result.runs[2].font.italic, true);
  assert.equal(result.runs[3].font.italic, false);
  assert.equal(result.runs[4].font.name, 'Other');
  assert.equal(result.runs[4].font.size, 18);
  assert.equal(result.runs[5].font.size, 12);
  assert.equal(result.runs[6].font.name, 'Base');
});
test('soft and line tags set draw and break state without exposing markup as characters', async () => {
  const parsed = parseTaggedText('<soft>x</soft><soft-line>\n</soft-line><line>\n</line><b>Y</b>', font);
  assert.equal(parsed.runs[0].soft, true);
  assert.equal(parsed.runs[0].font.shadow_displacement_x, 0);
  const provider = { prepare: async () => ({ measure: s => ({ width: s.length * 5, height: 12 }), lineHeight: () => 12 }) };
  const measured = await measureTaggedText(parsed, provider, null, null);
  assert.deepEqual(measured.characters.filter(c => !c.tag).map(c => c.break), ['maybe','soft','line','maybe']);
  assert.equal(measured.characters.length, parsed.sourceLength);
  await assert.rejects(() => measureTaggedText(parseTaggedText('<margin:10>word</margin>',font), provider,null,null), { code: 'text-tags' });
});
test('tag boundary matching distinguishes formatting tags from unknown names', () => {
  const parsed = parseTaggedText('<bold>A</bold><i-flavor>B</i-flavor><color:badcolor>C</color>',font);
  assert.equal(parsed.runs[0].font.bold, false);
  assert.equal(parsed.runs[1].font.italic, true);
  assert.deepEqual(parsed.runs[2].font.color.channels, font.color.channels);
  assert.equal(parsed.warnings.length, 1);
});

test('atom and error wrappers preserve source positions and nested decoration ranges', async () => {
  const source = '😀<atom-cardname>A<error-spelling:en_US><i>B</i></error-spelling:en_US>C</atom-cardname>D';
  const parsed = parseTaggedText(source, font);
  assert.deepEqual(parsed.unsupported, []);
  assert.deepEqual(parsed.runs.map(r => r.decorationIds), [[],[0],[0,1],[0],[]]);
  assert.deepEqual(parsed.decorations[0].background.channels, [210,210,210,255]);
  const provider = { prepare: async () => ({ measure: s => ({ width: [...s].length * 5, height: 12 }), lineHeight: () => 12 }) };
  const result = await measureTaggedText(parsed, provider, null, null);
  assert.equal(result.characters.length, [...source].length);
  const bIndex = [...source.slice(0, source.indexOf('B'))].length;
  assert.equal(result.characters[bIndex].width, 5);
  assert.equal(result.characters[bIndex-1].tag, true);
  assert.equal(result.characters.filter(c => !c.tag).length, 5);
  assert.equal(result.decorations[1].sourceStart, [...source.slice(0, source.indexOf('<i>'))].length);
});

test('keyword placeholders render bracketed separator-colored text at native source positions',async()=>{
  const source='A<atom-kwpph>value</atom-kwpph>Z';
  const parsed=parseTaggedText(source,font);
  assert.deepEqual(parsed.unsupported,[]);
  assert.deepEqual(parsed.runs.map(r=>r.text),['A','‹value›','Z']);
  assert.equal(parsed.runs[1].soft,false);
  assert.deepEqual(parsed.runs[1].font.color.channels,font.separator_color.channels);
  assert.equal(parsed.runs[1].font.shadow_displacement_x,0);
  assert.equal(parsed.runs[1].sourceStart,source.indexOf('value')-1);
  const measured=await measureTaggedText(parsed,{prepare:async()=>({measure:s=>({width:s.length*5,height:12}),lineHeight:()=>12})},null,null);
  assert.equal(measured.characters.filter(c=>!c.tag).length,9);
  assert.deepEqual(parsed.runs[2].font.color.channels,font.color.channels);
});
