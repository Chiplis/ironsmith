import test from 'node:test';
import assert from 'node:assert/strict';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { loadMseTemplate, MseTemplateContext } from '../src/lib/mse/template.mjs';
import { force, text } from '../src/lib/mse/script-values.mjs';

const files = {
  'example.mse-game/game': `init script: suffix := { set.label }
set field:
\ttype: text
\tname: label
\tinitial: !
card field:
\ttype: text
\tname: name
\tdefault: "Default"
\tscript: input + suffix()
card field:
\ttype: text
\tname: text
\tdefault: card.name + " rules"
`,
  'example.mse-style/style': `game: example
card width: 375
card height: 523
init script: title_left := { if styling.inset then 20 else 10 }
styling field:
\ttype: boolean
\tname: inset
\tinitial: yes
card style:
\tname:
\t\tleft: {title_left()}
\t\ttop: 30
\t\twidth: {375 - card_style.name.left*2}
\t\theight: 20
\t\tz index: 2
\t\tfont:
\t\t\tname: Beleren
\ttext:
\t\tleft: {card_style.name.left}
\t\ttop: {card_style.name.top + card_style.name.height + 10}
\t\twidth: {card_style.name.width}
\t\theight: 200
\t\tz index: 1
`,
};
const store = () => new MsePackageStore({ readText: async path => { if (!(path.key in files)) throw new Error(path.key); return files[path.key]; } });

test('game and stylesheet scripts drive supplied/default fields and cross-field geometry', async () => {
  const template = await loadMseTemplate(store(), 'example.mse-style');
  const context = new MseTemplateContext(template, { card: { name: 'Live' } }).initialize();
  assert.equal(text(context.field('card', 'name')), 'Live!');
  assert.equal(text(context.field('card', 'text')), 'Live! rules');
  const layout = context.layout();
  assert.deepEqual([layout.width, layout.height], [375, 523]);
  assert.deepEqual(layout.fields.map(field => field.name), ['text', 'name']);
  assert.equal(layout.fields[0].properties.width, 335);
  assert.equal(layout.fields[0].properties.top, 60);
  assert.equal(force(context.field('card', 'name')), 'Live!');
  assert.equal(context.field('card', 'name').isDefault, false);
  assert.equal(text(new MseTemplateContext(template).initialize().field('card', 'name')), 'Default!');
});

test('a fresh context recalculates defaults and layout when card/styling data changes', async () => {
  const template = await loadMseTemplate(store(), 'example.mse-style');
  const context = new MseTemplateContext(template, { card: { name: 'Other' }, set: { label: '?' }, styling: { inset: 'no' } }).initialize();
  assert.equal(text(context.field('card', 'text')), 'Other? rules');
  assert.equal(context.property('name', 'width'), 355);
});

test('template field scripts expand game keywords and retain usage per field', async () => {
  const fixture = { ...files, 'example.mse-game/game': files['example.mse-game/game'] + `
keyword:
\tkeyword: Flying
\tmatch: Flying
\treminder: Flight.
card field:
\ttype: text
\tname: rules
\tscript: expand_keywords(input, combine:{keyword + " (" + reminder + ")"})
card field:
\ttype: text
\tname: other rules
\tscript: expand_keywords(input, combine:{keyword})
` };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => fixture[path.key] }), 'example.mse-style');
  const context = new MseTemplateContext(template, { card: { rules: 'Flying', other_rules: 'Flying' } }).initialize();
  assert.equal(text(context.field('card', 'rules')), '<kw-A>Flying (Flight.)</kw-A>');
  assert.equal(text(context.field('card', 'other_rules')), '<kw-A>Flying</kw-A>');
  assert.deepEqual([...context.keywordUsage.keys()], ['card.rules', 'card.other_rules']);
  assert.equal(context.runtime.run('keyword_usage()'), 'Flying, Flying');
});

test('extra card fields and geometry remain separate from same-named game fields', async () => {
  const fixture = { ...files, 'example.mse-style/style': files['example.mse-style/style'] + `
extra card field:
\ttype: text
\tname: name
\tdefault: card.name + " extra"
extra card field:
\ttype: choice
\tname: emblem
\tinitial: gold
extra card style:
\tname:
\t\tleft: {card_style.name.left + 2}
\t\twidth: 80
\t\theight: 12
\t\tz index: 4
\temblem:
\t\tleft: {extra_card_style.name.left}
\t\timage: {to_image(input + ".png")}
\t\tz index: 3
` };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => fixture[path.key] }), 'example.mse-style');
  const context = new MseTemplateContext(template, { card: { name: 'Main' } }).initialize();
  assert.equal(text(context.field('extra_card', 'name')), 'Main! extra');
  assert.equal(context.property('name', 'width', 'extra_card'), 80);
  assert.equal(context.property('name', 'width'), 335);
  assert.equal(context.property('emblem', 'left', 'extra_card'), 22);
  assert.equal(context.property('emblem', 'image', 'extra_card').parameters.path.file, 'gold.png');
  assert.deepEqual(context.layout().fields.map(field => field.group), ['card', 'card', 'extra_card', 'extra_card']);
  context.setField('extra_card', 'name', 'Changed');
  assert.equal(text(context.field('extra_card', 'name')), 'Changed');
  assert.equal(text(context.field('card', 'name')), 'Main!');
});

test('native automatic geometry derives edges, centers declared bounds, and anchors rotated corners', async () => {
  const fixture = { ...files, 'example.mse-style/style': `game: example
card width: 375
card height: 523
card style:
\tname:
\t\tleft: 10
\t\tright: 110
\t\twidth: 0
\t\ttop: 5
\t\tbottom: 25
\ttext:
\t\tright: 100
\t\twidth: 20
\t\tbottom: 80
\t\theight: 10
\t\tangle: 90
` };
  const load = () => loadMseTemplate(new MsePackageStore({ readText: async path => fixture[path.key] }), 'example.mse-style');
  const context = new MseTemplateContext(await load()).initialize();
  assert.equal(context.property('name', 'width'), 100);
  assert.equal(context.property('name', 'height'), 20);
  assert.equal(context.property('text', 'left'), 90);
  assert.equal(context.property('text', 'top'), 100);
  assert.equal(context.property('text', 'right'), 100);
  fixture['example.mse-style/style'] = fixture['example.mse-style/style'].replace('width: 0', 'width: {0}');
  const scripted = new MseTemplateContext(await load()).initialize();
  assert.equal(scripted.property('name', 'width'), 0);
  assert.equal(scripted.property('name', 'left'), 60);
  assert.equal(scripted.property('name', 'right'), 60);
  fixture['example.mse-style/style'] = fixture['example.mse-style/style'].replace('left: 10', 'left: 110.75').replace('right: 110', 'right: 10.5').replace('width: {0}', 'width: 20');
  const centered = new MseTemplateContext(await load()).initialize();
  assert.equal(centered.property('name', 'left'), 50.5);
  assert.equal(centered.property('name', 'right'), 70.5);
});

test('scripted font definitions preserve native defaults and update after field edits', async () => {
  const fixture = { ...files, 'example.mse-style/style': files['example.mse-style/style'].replace('name: Beleren', `name: {if styling.inset then "Beleren" else "Matrix"}
\t\t\tsize: {if styling.inset then 12.5 else 10}
\t\t\titalic name: Beleren Italic
\t\t\tweight: bold
\t\t\tstyle: italic
\t\t\tunderline: {styling.inset}
\t\t\tcolor: {if styling.inset then rgb(20,30,40) else "white"}
\t\t\tscale down to: 8
\t\t\tstroke radius: {if styling.inset then 0.5 else 0}`) };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => fixture[path.key] }), 'example.mse-style');
  const context = new MseTemplateContext(template).initialize();
  const font = context.font('name');
  assert.equal(font.name, 'Beleren');
  assert.equal(font.size, 12.5);
  assert.equal(font.scale_down_to, 8);
  assert.equal(font.max_stretch, 1);
  assert.equal(font.bold, true);
  assert.equal(font.italic, true);
  assert.equal(font.underline, true);
  assert.deepEqual(font.color.channels, [20,30,40,255]);
  assert.deepEqual(font.separator_color.channels, [0,0,0,128]);
  assert.equal(context.layout().fields.find(f => f.name === 'name').properties.font, font);
  context.setField('styling', 'inset', false);
  const edited = context.font('name');
  assert.equal(edited.name, 'Matrix');
  assert.equal(edited.size, 10);
  assert.equal(edited.stroke_radius, 0);
  assert.equal(edited.underline, false);
  assert.deepEqual(edited.color.channels, [255,255,255,255]);
});

test('template text fitting remeasures at the chosen scale and aligns within field bounds', async () => {
  const fixture = { ...files, 'example.mse-style/style': files['example.mse-style/style'].replace('name: Beleren', 'name: Beleren\n\t\t\tsize: 20\n\t\t\tscale down to: 5') };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => fixture[path.key] }), 'example.mse-style');
  const context = new MseTemplateContext(template).initialize();
  const scales = [];
  const metrics = {
    measure: (source, font, scale) => { scales.push(scale); return { width: source.length * 10 * scale, height: 40 * scale }; },
    lineHeight: (font, scale) => 40 * scale,
  };
  const result = context.fitTextRun('name', 'Test', metrics);
  assert.equal(result.scale, 0.5);
  assert.equal(result.height, 20);
  assert.equal(result.fits, true);
  assert.equal(result.lines[0].top, 0);
  assert.ok(scales.includes(1));
  assert.ok(scales.includes(0.5));
});

test('prepared text metrics drive script geometry and are invalidated by card edits', async () => {
  const fixture = { ...files, 'example.mse-style/style': `game: example
card width: 375
card height: 523
card style:
\tname:
\t\tleft: 0
\t\ttop: 0
\t\twidth: {max(10, card_style.name.content_width)}
\t\theight: 20
\t\tfont:
\t\t\tname: Fixture
\ttext:
\t\tleft: {card_style.name.content_width + 5}
\t\ttop: 30
\t\twidth: 300
\t\theight: 20
` };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => fixture[path.key] }), 'example.mse-style');
  const context = new MseTemplateContext(template, { card: { name: 'AB' } }).initialize();
  const fonts = { prepare: async () => ({ measure: s => ({width:s.length*5,height:10}), lineHeight:()=>10 }) };
  const prepared = await context.prepareTextLayouts(fonts, null);
  assert.ok(prepared.passes >= 2);
  assert.equal(prepared.renderingVerified, false);
  assert.equal(context.property('name','content_width'), 15);
  assert.equal(context.property('name','width'), 15);
  assert.equal(context.property('text','left'), 20);
  assert.equal(context.runtime.run('card_style.name.layout.lines.0.width'), 15);
  context.setField('card','name','ABCDE');
  assert.throws(() => force(context.property('name','content_width')), { code:'script-member' });
  await context.prepareTextLayouts(fonts,null);
  assert.equal(context.property('text','left'), 35);
  await assert.rejects(() => context.prepareTextLayouts(fonts, null, { maxPasses: 1 }), { code: 'layout-convergence' });
  assert.equal(context.textMeasurements, null);
});

test('superseded text preparation cannot erase a newer completed layout', async () => {
  const fixture = { ...files, 'example.mse-style/style': `game: example
card width: 375
card height: 523
card style:
\tname:
\t\tleft: 0
\t\ttop: 0
\t\twidth: 300
\t\theight: 20
` };
  const template = await loadMseTemplate(new MsePackageStore({ readText: async path => fixture[path.key] }), 'example.mse-style');
  const context = new MseTemplateContext(template, {card:{name:'AB'}}).initialize();
  const metrics = {measure:s=>({width:s.length*5,height:10}),lineHeight:()=>10};
  let release, entered;
  const ready = new Promise(resolve=>{entered=resolve;});
  const gate = new Promise(resolve=>{release=resolve;});
  let first = true;
  const older = context.prepareTextLayouts({prepare:async()=>{
    if(first){first=false;entered();await gate;} return metrics;
  }},null);
  const rejected = assert.rejects(older,{code:'layout-stale'});
  await ready;
  context.setField('card','name','ABCDE');
  const newer = await context.prepareTextLayouts({prepare:async()=>metrics},null);
  const revision = context.layoutRevision;
  release();
  await rejected;
  assert.equal(context.textMeasurements,newer.measurements);
  assert.equal(context.layoutRevision,revision);
  assert.equal(context.property('name','content_width'),30);
});

test('choice image lookup supports nested scripts, aliases, and explicit image overrides', async()=>{
  const fixture={...files,'example.mse-style/style':`game: example
card width: 375
card height: 523
card style:
\tname:
\t\tright: 100
\t\twidth:
\t\t\tscript:
\t\t\t\tif card.name == "basic land" then 20 else 0
\t\theight: 20
\t\tchoice images:
\t\t\tbasic land:
\t\t\t\tscript:
\t\t\t\t\t"frame.png"
\t\t\tcommon: common.png
\ttext:
\t\timage: explicit.png
\t\tchoice images:
\t\t\tcommon: ignored.png
`};
  const template=await loadMseTemplate(new MsePackageStore({readText:async path=>fixture[path.key]}),'example.mse-style');
  const context=new MseTemplateContext(template,{card:{name:'basic land'}}).initialize();
  // This fixture's game name script adds '!'; use its normalized stored result.
  context.cells.card.get('name').definition={...context.cells.card.get('name').definition,script:null};
  assert.equal(context.property('name','width'),20);
  assert.equal(context.property('name','left'),80);
  assert.equal(context.property('name','image'),'frame.png');
  context.setField('card','name','basic_land');
  assert.equal(context.property('name','image'),'frame.png');
  assert.equal(context.property('name','width'),0);
  assert.equal(context.property('name','left'),100);
  context.setField('card','name','missing');
  assert.equal(context.property('name','image'),null);
  assert.equal(context.property('text','image'),'explicit.png');
});

test('choice image measurements drive neighboring text geometry and update with the choice',async()=>{
  const fixture={
    'example.mse-game/game':'card field:\n\tname: rarity\n\ttype: choice\n\tchoice: common\n\tchoice: rare\ncard field:\n\tname: title\n\ttype: text\n',
    'example.mse-style/style':`game: example
card width: 100
card height: 100
card style:
\ttitle:
\t\ttop: 0
\t\tleft: {card_style.rarity.content_width + 5}
\t\twidth: 50
\t\theight: 20
\trarity:
\t\tleft: 0
\t\ttop: 0
\t\twidth: 40
\t\theight: 20
\t\tvisible: {card.rarity != "hidden"}
\t\trender style: image
\t\talignment: middle center
\t\tchoice images:
\t\t\tcommon: wide.png
\t\t\trare: tall.png
`};
  const template=await loadMseTemplate(new MsePackageStore({readText:async p=>fixture[p.key]}),'example.mse-style');
  const context=new MseTemplateContext(template).initialize();
  const renderer={render:async graph=>({width:graph.parameters.path.file==='wide.png'?80:10,height:40,hasAlpha:true,data:new Uint8ClampedArray((graph.parameters.path.file==='wide.png'?80:10)*40*4)})};
  const fonts={prepare:async()=>({measure:s=>({width:s.length*5,height:10}),lineHeight:()=>10})};
  await context.prepareTextLayouts(fonts,renderer);
  assert.equal(context.property('rarity','content_width'),40);
  assert.equal(context.property('title','left'),45);
  context.setField('card','rarity','rare');
  await context.prepareTextLayouts(fonts,renderer);
  assert.equal(context.property('rarity','content_width'),5);
  assert.equal(context.property('rarity','content_height'),20);
  assert.equal(context.property('title','left'),10);
  context.setField('card','rarity','hidden');
  await context.prepareTextLayouts(fonts,{render:async()=>{throw new Error('Hidden image was loaded');}});
  assert.equal(context.textMeasurements.get('card.rarity').result,null);
  assert.equal(context.property('rarity','content_width'),0);
});

test('choice images stretch to field dimensions by default, including square corner textures',async()=>{
  const fixture={
    'example.mse-game/game':'card field:\n\tname: corners\n\ttype: choice\n\tchoice: rounded\n',
    'example.mse-style/style':'game: example\ncard width: 375\ncard height: 523\ncard style:\n\tcorners:\n\t\tleft: 0\n\t\ttop: 0\n\t\twidth: 375\n\t\theight: 523\n\t\trender style: image\n\t\timage: color.png\n',
  };
  const template=await loadMseTemplate(new MsePackageStore({readText:async p=>fixture[p.key]}),'example.mse-style');
  const context=new MseTemplateContext(template).initialize();
  const measured=await context.measureChoiceImage('corners',{render:async()=>({width:1,height:1,hasAlpha:false,data:new Uint8ClampedArray([255,255,255,255])})});
  assert.equal(measured.width,375);assert.equal(measured.height,523);
  assert.deepEqual([...measured.image.data.slice(-4)],[255,255,255,255]);
});

test('render layouts skip inactive fields without hiding errors in active fields',async()=>{
  const fixture={
    'example.mse-game/game':'card field:\n\tname: overlay\n\ttype: choice\n\tchoice: off\n\tchoice: on\n',
    'example.mse-style/style':'game: example\ncard width: 10\ncard height: 10\ncard style:\n\toverlay:\n\t\tleft: 0\n\t\ttop: 0\n\t\twidth: 10\n\t\theight: 10\n\t\tvisible: {card.overlay == "on"}\n\t\trender style: image\n\t\timage: {card.nonexistent}\n',
  };
  const template=await loadMseTemplate(new MsePackageStore({readText:async p=>fixture[p.key]}),'example.mse-style');
  const context=new MseTemplateContext(template).initialize();
  assert.equal(context.layout({visibleOnly:true}).fields.length,0);
  assert.throws(()=>context.layout(),{code:'script-member'});
  context.setField('card','overlay','on');
  assert.throws(()=>context.layout({visibleOnly:true}),{code:'script-member'});
});
