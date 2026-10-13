import test from 'node:test';
import assert from 'node:assert/strict';
import {MseScriptLoader} from '../src/lib/mse/script-loader.mjs';
import {MseRuntime} from '../src/lib/mse/script-runtime.mjs';
import {MsePackageStore} from '../src/lib/mse/packages.mjs';
import {loadMseTemplate,MseTemplateContext} from '../src/lib/mse/template.mjs';
import {compileFontDefinition,evaluateFontDefinition} from '../src/lib/mse/font-definition.mjs';
import {parseMseDocument} from '../src/lib/mse/document.mjs';

test('scriptable scalars honor expression prefixes without changing literal or interpolated values',async()=>{
  const loader=new MseScriptLoader({}),runtime=new MseRuntime();
  for(const [source,expected] of [['script: 20 + 2',22],['script:\n\t20 + 2',22],['{20 + 2}',22],['frame-{2}.png','frame-2.png'],['frame.png','frame.png']]){
    assert.equal(runtime.evaluate(await loader.compileScriptable(source)),expected,source);
  }
  assert.equal(runtime.evaluate(await loader.compile('script: literal',{stringMode:true})),'script: literal');
});

test('choice images accept multiline interpolations and inline expression prefixes',async()=>{
  const files={
    'example.mse-game/game':'card field:\n\tname: frame\n\ttype: choice\n\tchoice: common\n\tchoice: rare\n',
    'example.mse-style/style':'game: example\ncard style:\n\tframe:\n\t\twidth: script: 20 + 2\n\t\theight:\n\t\t\tscript:\n\t\t\t\t30 + 3\n\t\tchoice images:\n\t\t\tcommon:\n\t\t\t\t{\n\t\t\t\t   "common.png"\n\t\t\t\t}\n\t\t\trare: script: "rare.png"\n',
  };
  const template=await loadMseTemplate(new MsePackageStore({readText:async p=>files[p.key]}),'example.mse-style');
  const context=new MseTemplateContext(template).initialize();
  assert.equal(context.property('frame','width'),22);
  assert.equal(context.property('frame','height'),33);
  assert.equal(context.property('frame','image'),'common.png');
  context.setField('card','frame','rare');
  assert.equal(context.property('frame','image'),'rare.png');
});

test('font expressions support script prefixes for numeric, color, and family properties',async()=>{
  const loader=new MseScriptLoader({}),runtime=new MseRuntime();
  const record=parseMseDocument('name: script: "Beleren"\nsize: script: 10 + 2\ncolor: script: "red"\n');
  const font=evaluateFontDefinition(await compileFontDefinition(record,loader),p=>runtime.evaluate(p));
  assert.equal(font.name,'Beleren');assert.equal(font.size,12);
  assert.deepEqual(font.color.channels,[255,0,0,255]);
});
