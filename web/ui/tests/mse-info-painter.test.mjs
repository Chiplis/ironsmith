import test from 'node:test';
import assert from 'node:assert/strict';
import {paintInfoField,infoCaption} from '../src/lib/mse/info-painter.mjs';
import {color} from '../src/lib/mse/color-values.mjs';
import {parseMseDocument} from '../src/lib/mse/document.mjs';

test('info fields paint expanded backgrounds and align plain text inside padding',async()=>{
  const calls=[],ctx={save(){},restore(){},fillRect(...v){calls.push(['background',...v,this.fillStyle]);}};
  const fonts={prepare:async font=>{
    assert.equal(font.stroke_radius,0);assert.equal(font.shadow_displacement_x,0);
    return {measure:()=>({width:20,height:10}),draw:(_ctx,...v)=>calls.push(['text',...v])};
  }};
  await paintInfoField(ctx,'<b>literal</b>',{color:color('black'),stroke_radius:3,shadow_displacement_x:5},fonts,
    {width:100,height:50,alignment:'bottom right',paddingLeft:3,paddingRight:7,paddingTop:4,paddingBottom:6,background:'red'});
  assert.deepEqual(calls[0].slice(0,5),['background',-2,-2,104,54]);
  assert.deepEqual(calls[1],['text','<b>literal</b>',73,34,1]);
});
test('empty info text uses its caption without capitalizing supplied content',async()=>{
  let drawn;
  const ctx={save(){},restore(){},fillRect(){}};
  const fonts={prepare:async()=>({measure:()=>({width:10,height:5}),draw:(_ctx,...v)=>{drawn=v;}})};
  await paintInfoField(ctx,'',{color:color('black')},fonts,{width:20,height:20,caption:'lowercase caption'});
  assert.deepEqual(drawn,['lowercase caption',2,2,1]);
  assert.equal(infoCaption(parseMseDocument('name: status_text\n')),'Status text');
  assert.equal(infoCaption(parseMseDocument('name: status_text\ncaption: Custom label\n')),'Custom label');
});
