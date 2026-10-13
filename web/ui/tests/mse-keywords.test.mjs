import test from 'node:test';
import assert from 'node:assert/strict';
import { parseMseDocument } from '../src/lib/mse/document.mjs';
import { parseMseScript } from '../src/lib/mse/script-parser.mjs';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';
import { loadMseKeywords, keywordBuiltins, cleanKeywordText } from '../src/lib/mse/keywords.mjs';

const definitions = `keyword parameter type:
\tname: number
\tmatch: [0-9]+
keyword parameter type:
\tname: cost
\tmatch: [—-][0-9]+
\tseparator before is: [—-]
\tseparator script: "—"
\toptional: false
keyword:
\tkeyword: Flying
\tmatch: Flying
\tmode: core
\treminder: Takes flight.
keyword:
\tkeyword: Flying high
\tmatch: Flying high
\treminder: Higher.
keyword:
\tkeyword: Level
\tmatch: Level <atom-param>number</atom-param>
\treminder: Level {to_int(param1) + 1}.
keyword:
\tkeyword: Cost
\tmatch: Cost—<atom-param>cost</atom-param>
\treminder: Pay {param1.value}; separator {param1.separator_before}.
`;
async function vm(custom = '') {
  const game = parseMseDocument(definitions);
  const keywords = await loadMseKeywords(game, { compile: async (source, options) => parseMseScript(source, options) }, parseMseDocument(custom).all('keyword').map(entry => entry.record()));
  return new MseRuntime({ builtins: { ...coreBuiltins(), ...keywordBuiltins(keywords) } });
}
const expand = (runtime, input, options = '') => {
  runtime.scope.set('sample', input);
  return runtime.run(`expand_keywords(sample, combine:{keyword + if expand then " (" + reminder + ")" else ""}${options})`);
};

test('keyword matches prioritize longest, preserve formatting and skip atoms', async () => {
  const runtime = await vm();
  assert.equal(expand(runtime, 'Flying high'), '<kw-A>Flying high (Higher.)</kw-A>');
  assert.equal(expand(runtime, '<b>Flying</b>'), '<kw-A><b>Flying</b> (Takes flight.)</kw-A>');
  // Pinned native traversal consumes opening tags after a preceding character.
  assert.equal(expand(runtime, 'x <b>Flying</b>'), 'x <b><kw-A>Flying</b> (Takes flight.)</kw-A>');
  assert.equal(expand(runtime, '<atom>Flying</atom> butterfly Flying'), '<atom>Flying</atom> butterfly <kw-A>Flying (Takes flight.)</kw-A>');
  assert.equal(expand(runtime, 'Flyingness Flying('), 'Flyingness Flying(');
});

test('explicit keyword visibility survives reprocessing and conditions can reject matches', async () => {
  const runtime = await vm();
  assert.equal(expand(runtime, '<kw-0>Flying<atom-reminder> stale <atom>x</atom></atom-reminder></kw-0>'), '<kw-0>Flying</kw-0>');
  assert.equal(expand(runtime, '<kw-1>Flying</kw-1>', ', default_expand:false'), '<kw-1>Flying (Takes flight.)</kw-1>');
  assert.equal(expand(runtime, 'flying Flying', ', condition:{correct_case}, default_expand:false'), 'flying <kw-a>Flying</kw-a>');
  assert.equal(cleanKeywordText('<param-number>2</param-number><atom-kwpph>number</atom-kwpph>'), '2');
});

test('keyword parameter objects expose values, separators and numeric conversion', async () => {
  const runtime = await vm();
  assert.equal(expand(runtime, 'Level 2'), '<kw-A>Level <param-number>2</param-number> (Level 3.)</kw-A>');
  assert.equal(expand(runtime, 'Cost-3'), '<kw-A>Cost—<param-cost>3</param-cost> (Pay 3; separator —.)</kw-A>');
  assert.equal(expand(runtime, 'Level ', ', condition:{not used_placeholders}'), 'Level ');
  assert.equal(expand(runtime, 'Flying Flying'), '<kw-A>Flying (Takes flight.)</kw-A> <kw-A>Flying (Takes flight.)</kw-A>');
  assert.equal(runtime.run('keyword_usage()'), 'Flying, Flying');
  assert.equal(runtime.run('keyword_usage(unique:true)'), 'Flying');
  expand(runtime, 'Nothing');
  assert.equal(runtime.run('keyword_usage()'), '');
});

test('set keywords precede longer game matches and placeholder state is available to callbacks', async () => {
  const runtime = await vm(`keyword:\n\tkeyword: Flying\n\tmatch: Flying\n\treminder: Custom.\nkeyword:\n\tkeyword: Count\n\tmatch: Count <atom-param>number</atom-param>\n\treminder: {if used_placeholders then "missing" else to_string(length(param1))}`);
  assert.equal(expand(runtime, 'Flying high'), '<kw-A>Flying (Custom.)</kw-A> high');
  assert.equal(expand(runtime, 'Count '), '<kw-A>Count <param-number><atom-kwpph>number</atom-kwpph></param-number> (missing)</kw-A>');
  assert.equal(expand(runtime, 'Count 123'), '<kw-A>Count <param-number>123</param-number> (3)</kw-A>');
});

test('optional leading parameters cannot introduce keywords at word ends',async()=>{
  const game=parseMseDocument(`keyword parameter type:
\tname: mill
\tmatch: mills?
keyword parameter type:
\tname: word
\tmatch: [A-Za-z0-9]+
keyword:
\tkeyword: Mill
\tmatch: <atom-param>mill</atom-param> <atom-param>word</atom-param> cards
\treminder: Put cards into the graveyard.
`);
  const keywords=await loadMseKeywords(game,{compile:async(source,options)=>parseMseScript(source,options)});
  const runtime=new MseRuntime({builtins:{...coreBuiltins(),...keywordBuiltins(keywords)}});
  for(const input of ['draw two cards','draw<b> two cards</b>',' two cards','redraw two cards'])assert.equal(expand(runtime,input),input);
  assert.match(expand(runtime,'mill two cards'),/^<kw-A><param-mill>mill<\/param-mill>/);
  assert.match(expand(runtime,'Then mill two cards.'),/^Then <kw-A>/);
});
