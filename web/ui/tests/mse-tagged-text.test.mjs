import test from 'node:test';
import assert from 'node:assert/strict';
import { parseTaggedText, measureTaggedText } from '../src/lib/mse/tagged-text.mjs';
import { color } from '../src/lib/mse/color-values.mjs';
const font = { name: 'Base', italic_name: 'Italic', size: 12, scale_down_to: 8, color: color('black'), separator_color: color('gray'), shadow_displacement_x: 2, shadow_displacement_y: 3 };

test('code formatting preserves native syntax-color precedence and restores surrounding fonts',async()=>{
  const {resolveFontFace}=await import('../src/lib/mse/font-definition.mjs');
  const parsed=parseTaggedText('<i>A<code>B<code-str>C</code-str><code-kw>D</code-kw><color:green>E</color><soft>F</soft></code>G</i><code-str>H</code-str>',font);
  assert.deepEqual(parsed.unsupported,[]);
  assert.deepEqual(parsed.runs.map(r=>r.font.color.channels),[
    [0,0,0,255],[128,0,0,255],[128,0,0,255],[158,100,0,255],[0,255,0,255],[128,128,128,255],[0,0,0,255],[0,0,100,255],
  ]);
  assert.deepEqual(resolveFontFace(parsed.runs[1].font),{name:'Courier New',bold:false,italic:false});
  assert.deepEqual(resolveFontFace(parsed.runs[3].font),{name:'Courier New',bold:true,italic:false});
  assert.equal(resolveFontFace(parsed.runs[6].font).name,'Italic');
  assert.equal(parsed.runs[5].font.shadow_displacement_x,0);
});

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
});

test('parameter placeholders bracket text and cycle native colors ahead of explicit color tags',async()=>{
  const parsed=parseTaggedText('<color:red><atom-param>A</atom-param><atom-param>B</atom-param></color><ref-param 3>C</ref-param 3><ref-param:1>D</ref-param:1>E',font);
  assert.deepEqual(parsed.unsupported,[]);
  assert.deepEqual(parsed.runs.map(r=>r.text),['‹A›','‹B›','C','D','E']);
  assert.deepEqual(parsed.runs.map(r=>r.font.color.channels),[[0,170,0,255],[0,0,200,255],[200,0,100,255],[200,200,0,255],[0,0,0,255]]);
  const fonts={prepare:async()=>({measure:s=>({width:[...s].length*5,height:12}),lineHeight:()=>12})};
  const measured=await measureTaggedText(parsed,fonts,null,null);
  assert.equal(measured.characters.filter(c=>!c.tag).length,9);
  assert.equal(measured.characters.length,parsed.sourceLength);
});

test('reference parameter indexing follows native tag boundaries and six-color wrapping',()=>{
  const parsed=parseTaggedText('<ref-param 7>A</ref-param 7><ref-param123>B</ref-param123><ref-param 2>C<i>D</i></ref-param 2>',font);
  assert.deepEqual(parsed.runs.map(r=>r.font.color.channels),[[0,170,0,255],[0,0,0,255],[0,0,200,255],[200,0,100,255]]);
  assert.equal(parsed.runs[3].font.italic,true);
});

test('margin scopes accumulate per clause and retain native numeric substring behavior',async()=>{
  const parsed=parseTaggedText('<margin:10 :20 :3>A<margin:5 :6 :2>B\nC</margin>\nD</margin>\nE',font);
  assert.deepEqual(parsed.clauses.map(c=>[c.marginLeft||0,c.marginRight||0,c.marginTop||0]),[[15,26,5],[15,26,5],[10,20,3],[0,0,0]]);
  assert.deepEqual(parsed.unsupported,[]);
  const compact=parseTaggedText('<margin:10:20:3>X',font).clauses[0];
  assert.deepEqual([compact.marginLeft,compact.marginRight,compact.marginTop],[1,2,3]);
  const fonts={prepare:async()=>({measure:s=>({width:s.length*5,height:12}),lineHeight:()=>12})};
  const measured=await measureTaggedText(parsed,fonts,null,null,{scale:0.5});
  assert.equal(measured.characters[0].marginLeft,15,'native margins remain absolute at smaller font scales');
});

test('margins constrain wrapping, add clause spacing, and retain left indent across soft breaks',async()=>{
  const {wrapTextCharacters}=await import('../src/lib/mse/text-wrap.mjs');
  const fonts={prepare:async()=>({measure:s=>({width:s.length*5,height:12}),lineHeight:()=>12})};
  const parsed=parseTaggedText('<margin:10 :5 :3>AA BB\nC</margin>\nD',font);
  const measured=await measureTaggedText(parsed,fonts,null,null);
  const result=wrapTextCharacters(measured.characters,{width:35,height:100});
  assert.deepEqual(result.lines.map(l=>[l.positions[0],l.top,l.break]),[[10,3,'wrap'],[10,15,'hard'],[10,30,'hard'],[0,42,'no']]);
  const soft=await measureTaggedText(parseTaggedText('<margin:10 :5 :3>A<soft-line>\n</soft-line><margin:20 :7 :4>B</margin></margin>\nC',font),fonts,null,null);
  const lines=wrapTextCharacters(soft.characters,{width:100,height:100}).lines;
  assert.deepEqual(lines.map(l=>[l.positions[0],l.top,l.marginRight]),[[10,3,5],[10,22,12],[0,34,0]]);
});

test('aligned text retains the first top margin and a trailing empty clause inherits margins',async()=>{
  const {layoutTaggedText}=await import('../src/lib/mse/tagged-layout.mjs');
  const fonts={prepare:async()=>({measure:s=>({width:s.length*5,height:12}),lineHeight:()=>12})};
  const parsed=parseTaggedText('<margin:10 :5 :3>A\n',font);
  const options={width:100,height:100,alignment:8192|257,multiLine:true};
  const top=await layoutTaggedText(parsed,fonts,null,null,options);
  assert.deepEqual(top.lines.map(l=>[l.positions[0],l.top]),[[10,3],[10,18]]);
  const middle=await layoutTaggedText(parsed,fonts,null,null,{...options,alignment:8192|513});
  assert.equal(middle.lines[0].top,3+(100-27)/2);
  assert.equal(middle.lines[1].top-middle.lines[0].top,15);
});

test('list bullets establish hanging indents for wraps and repeat at the original bullet column',async()=>{
  const {wrapTextCharacters}=await import('../src/lib/mse/text-wrap.mjs');
  const fonts={prepare:async()=>({measure:s=>({width:[...s].length*5,height:12}),lineHeight:()=>12})};
  const source='<li><bullet>• </bullet>AA BB CC<soft-line>\n</soft-line><bullet>• </bullet>DD EE</li>\nZ';
  const parsed=parseTaggedText(source,font);
  assert.deepEqual(parsed.unsupported,[]);assert.deepEqual(parsed.warnings,[]);
  const measurement=await measureTaggedText(parsed,fonts,null,null);
  const result=wrapTextCharacters(measurement.characters,{width:30,height:150});
  assert.deepEqual(result.lines.map(l=>l.positions[0]),[0,10,10,0,10,0]);
  assert.deepEqual(result.lines.map(l=>l.break),['wrap','wrap','soft','wrap','hard','no']);
  assert.equal(result.lines[3].bullet,true);
  assert.equal(result.lines[4].marginLeftAfterBullet,10,'repeated bullets do not accumulate indentation');
});

test('bullet markers use Unicode source positions, retain prefixes, and warn outside lists',async()=>{
  const {wrapTextCharacters}=await import('../src/lib/mse/text-wrap.mjs');
  const fonts={prepare:async()=>({measure:s=>({width:[...s].length*5,height:12}),lineHeight:()=>12})};
  const parsed=parseTaggedText('😀<bullet>• </bullet>AB CD',font);
  assert.equal(parsed.paragraphs[0].bulletStart,1);
  assert.deepEqual(parsed.warnings,['<bullet> outside <li> tag','<bullet> outside <li> tag']);
  const measurement=await measureTaggedText(parsed,fonts,null,null);
  const result=wrapTextCharacters(measurement.characters,{width:30,height:100});
  assert.equal(result.lines[0].marginLeftBeforeBullet,5);
  assert.equal(result.lines[0].marginLeftAfterBullet,15);
  assert.equal(result.lines[1].positions[0],15);
});

test('alignment tags affect the current paragraph and restore the stack at paragraph boundaries',async()=>{
  const {layoutTaggedText}=await import('../src/lib/mse/tagged-layout.mjs');
  const parsed=parseTaggedText('<align:right>A<align:center>B</align>\nC</align>\nD',font);
  const fonts={prepare:async()=>({measure:s=>({width:s.length*5,height:12}),lineHeight:()=>12})};
  const result=await layoutTaggedText(parsed,fonts,null,null,{width:100,height:100,alignment:8192|257,multiLine:true});
  const xFor=content=>{
    const run=result.measurement.runs.find(r=>r.text.includes(content));
    const index=run.characterStart+[...run.text.slice(0,run.text.indexOf(content))].length;
    const line=result.lines.find(l=>index>=l.start&&index<l.end);
    return line.positions[index-line.start];
  };
  assert.equal(xFor('A'),45);assert.equal(xFor('B'),50);
  assert.equal(xFor('C'),95);assert.equal(xFor('D'),0);
  const soft=parseTaggedText('<align:right>A<soft-line>\n</soft-line>B</align>\nC',font);
  assert.equal(soft.paragraphs.length,2);
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
