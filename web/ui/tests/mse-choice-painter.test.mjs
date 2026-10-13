import test from 'node:test';
import assert from 'node:assert/strict';
import {paintChoiceLabel} from '../src/lib/mse/choice-painter.mjs';
import {MseColor} from '../src/lib/mse/script-values.mjs';

test('choice labels use native capitalization, single-line alignment and image margin',async()=>{
  const calls=[],ctx={save(){},restore(){}},font={color:new MseColor(10,20,30)};
  const fonts={prepare:async()=>({measure:()=>({width:20,height:10}),draw:(ctx,...args)=>calls.push(args)})};
  await paintChoiceLabel(ctx,'rare',font,fonts,{width:100,height:40,alignment:'bottom right'});
  assert.deepEqual(calls.pop(),['Rare',80,30,1]);
  await paintChoiceLabel(ctx,'rare',font,fonts,{width:100,height:40,alignment:'bottom right',withImage:true,imageWidth:16});
  assert.deepEqual(calls.pop(),['Rare',18,15,1]);
  await paintChoiceLabel(ctx,'rare',font,fonts,{width:100,height:40,withImage:true});
  assert.deepEqual(calls.pop(),['Rare',0,15,1]);
  await paintChoiceLabel(ctx,'',font,fonts,{width:100,height:40});
  assert.equal(calls.length,0);
  await paintChoiceLabel(ctx,'rare',font,fonts,{width:100,height:40,translate:()=> 'Localized'});
  assert.equal(calls.pop()[0],'Localized');
});
