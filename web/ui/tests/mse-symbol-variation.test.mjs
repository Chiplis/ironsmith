import test from 'node:test';
import assert from 'node:assert/strict';
import {parseMseDocument} from '../src/lib/mse/document.mjs';
import {readSymbolVariation,symbolVariationColor,filterSymbolRaster} from '../src/lib/mse/symbol-variation.mjs';
test('symbol gradients mirror about their center and preserve border/outside classification',()=>{
  const variation=readSymbolVariation(parseMseDocument('name: rare\nfill type: linear gradient\nfill color 1: rgb(200,100,0)\nfill color 2: rgb(0,0,0)\nborder color 1: white\nborder color 2: black\n'));
  assert.deepEqual(symbolVariationColor(variation,0.5,0.5,'inside'),[200,100,0,255]);
  assert.deepEqual(symbolVariationColor(variation,0,0,'inside'),symbolVariationColor(variation,1,1,'inside'));
  assert.deepEqual(symbolVariationColor(variation,0.5,0.5,'border'),[255,255,255,255]);
  assert.deepEqual(symbolVariationColor(variation,0.5,0.5,'outside'),[0,0,0,0]);
  const solid=readSymbolVariation(parseMseDocument('fill type: solid\nfill color: red\nborder color: white\n'));
  const source={width:3,height:1,hasAlpha:true,data:new Uint8ClampedArray([0,0,0,255,255,255,255,255,0,255,0,255])};
  assert.deepEqual([...filterSymbolRaster(source,solid).data],[255,0,0,255,255,255,255,255,0,0,0,0]);
  assert.equal(source.data[11],255);
});

import {MseRuntime} from '../src/lib/mse/script-runtime.mjs';
import {coreBuiltins} from '../src/lib/mse/script-builtins.mjs';
import {imageBuiltins} from '../src/lib/mse/image-builtins.mjs';
import {MseFieldValue} from '../src/lib/mse/script-values.mjs';
import {MseImageRenderer} from '../src/lib/mse/image-renderer.mjs';
test('symbol variation scripts distinguish local symbol fields from package filenames',async()=>{
  const vm=new MseRuntime({builtins:{...coreBuiltins(),...imageBuiltins('example.mse-style')}});
  const variation=readSymbolVariation(parseMseDocument('name: common\nfill type: solid\nfill color: black\nborder color: white'));
  const cell={value:'custom-symbol',lastUpdate:5,definition:{record:parseMseDocument('type: symbol')}};
  vm.scope.set('fixture',new MseFieldValue(null,true,{cell,group:'set',name:'symbol',context:{template:{symbolVariations:new Map([['symbol',new Map([['common',variation]])]])}}}));
  const local=vm.run('symbol_variation(symbol:fixture,variation:"common")');
  assert.equal(local.parameters.local,true);assert.equal(local.parameters.filename,'custom-symbol');assert.equal(local.parameters.age,5);
  assert.equal(local.parameters.variation,variation);
  const packaged=vm.run('symbol_variation(symbol:"icons/set.mse-symbol",border_radius:0.1,fill_color:"red",border_color:"white")');
  assert.equal(packaged.parameters.local,false);assert.equal(packaged.parameters.path.key,'example.mse-style/icons/set.mse-symbol');
  await assert.rejects(new MseImageRenderer().render(local),{code:'symbol-renderer'});
  let received;
  const renderer=new MseImageRenderer({renderSymbol:async p=>{received=p;return {width:1,height:1,data:new Uint8ClampedArray(4),hasAlpha:true};}});
  await renderer.render(local);assert.equal(received,local.parameters);
});
