import {text} from '../src/lib/mse/script-values.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {MseCardRenderer} from '../src/lib/mse/card-renderer.mjs';
import {MsePackageStore} from '../src/lib/mse/packages.mjs';
const documents={
  'example.mse-game/game':'card field:\n\tname: image\n\ttype: image\n',
  'example.mse-style/style':'game: example\ncard width: 10\ncard height: 12\ncard style:\n\timage:\n\t\tleft: 0\n\t\ttop: 0\n\t\twidth: 10\n\t\theight: 12\n',
};
function canvas(width=1,height=1){const calls=[];return {width,height,calls,getContext:()=>({fillRect(){},save(){},restore(){},translate(){},drawImage(...args){calls.push(args);}})};}
function setup(loadCardImage){return new MseCardRenderer({store:new MsePackageStore({readText:async p=>documents[p.key]}),fonts:{},imageRenderer:{maxPixels:1000},createCanvas:canvas,loadCardImage});}
const request=image=>({style:'example.mse-style',context:{card:{image}}});
test('render service coalesces templates and isolates contexts between cards',async()=>{
  const r=setup(async()=>null);
  const [a,b]=await Promise.all([r.template('example.mse-style'),r.template('example.mse-style')]);
  assert.equal(a,b);
  const x=await r.render(request('first')),y=await r.render(request('second'));
  assert.notEqual(x.context,y.context);assert.equal(x.context.template,y.context.template);
  assert.equal(x.width,10);assert.equal(x.height,12);
});
test('a late render cannot overwrite a newer result on the same target',async()=>{
  let release,entered;const gate=new Promise(resolve=>{release=resolve;});const ready=new Promise(resolve=>{entered=resolve;});
  const r=setup(async value=>{if(value==='slow'){entered();await gate;}return null;});
  const target=canvas();const older=r.renderTo(target,request('slow'));
  const rejected=assert.rejects(older,{code:'render-superseded'});
  await ready;const newer=await r.renderTo(target,request('fast'));release();await rejected;
  assert.equal(target.calls.length,1);assert.equal(target.calls[0][0],newer.canvas);
});
test('cancellation and disposal prevent in-flight work from changing a visible card',async()=>{
  for(const action of ['abort','dispose']){
    let release,entered;const gate=new Promise(resolve=>{release=resolve;});const ready=new Promise(resolve=>{entered=resolve;});
    const r=setup(async()=>{entered();await gate;return null;}),target=canvas(7,8),controller=new AbortController();
    const pending=r.renderTo(target,request('slow'),{signal:controller.signal});
    const rejected=assert.rejects(pending,{code:action==='abort'?'render-aborted':'renderer-disposed'});
    await ready;if(action==='abort')controller.abort();else r.dispose();release();await rejected;
    assert.equal(target.width,7);assert.equal(target.height,8);assert.equal(target.calls.length,0);
  }
});

test('DFC rendering initializes linked faces with independent styles before rendering the selected face',async()=>{
  const files={
    'example.mse-game/game':'card field:\n\ttype: text\n\tname: name\ncard field:\n\ttype: text\n\tname: counterpart\n\tdefault: if get_back_face(card) != nil then get_back_face(card).name else get_front_face(card).name\n',
    'magic-m15-altered.mse-style/style':'game: example\ncard width: 10\ncard height: 12\n',
    'magic-m15-mainframe-planeswalker.mse-style/style':'game: example\ncard width: 10\ncard height: 12\n',
    'magic-m15-planeswalker.mse-style/style':'game: example\ncard width: 10\ncard height: 12\n',
  };
  const r=new MseCardRenderer({store:new MsePackageStore({readText:async p=>files[p.key]}),fonts:{},imageRenderer:{},fieldUpdateMode:'ordered'});
  r.renderContext=async context=>context;
  const snapshot={layout:'transform',card_faces:[{name:'Day',type_line:'Creature'},{name:'Night',type_line:'Planeswalker'}]};
  const front=await r.render({style:'magic-m15-altered.mse-style',snapshot});
  assert.equal(text(front.field('card','counterpart')),'Night');
  assert.equal(text(front.runtime.run('get_back_face(card).name')),'Night');
  assert.equal(front.runtime.run('length(set.cards)'),2);
  const back=await r.render({style:'magic-m15-planeswalker.mse-style',snapshot,adapter:{faceIndex:1}});
  assert.equal(text(back.field('card','name')),'Night');
  assert.equal(text(back.field('card','counterpart')),'Day');
  assert.equal(text(back.runtime.run('get_front_face(card).name')),'Day');
  assert.equal(back.template.name,'magic-m15-planeswalker.mse-style');
  r.dispose();
});

test('legacy front/back region templates retain both numbered faces',async()=>{
  const files={
    'example.mse-game/game':'card field:\n\ttype: text\n\tname: name\ncard field:\n\ttype: text\n\tname: name 2\n',
    'legacy.mse-style/style':'game: example\ncard width: 20\ncard height: 12\ncard region:\n\tname: front\ncard region:\n\tname: back\n',
  };
  const r=new MseCardRenderer({store:new MsePackageStore({readText:async p=>files[p.key]}),fonts:{},imageRenderer:{}});
  r.renderContext=async context=>context;
  const context=await r.render({style:'legacy.mse-style',snapshot:{layout:'transform',card_faces:[{name:'Day'},{name:'Night'}]},adapter:{faceIndex:0}});
  assert.equal(text(context.field('card','name')),'Day');
  assert.equal(text(context.field('card','name_2')),'Night');
  assert.equal(context.runtime.run('length(set.cards)'),1);
});
