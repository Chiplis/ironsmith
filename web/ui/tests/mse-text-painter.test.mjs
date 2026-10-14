import test from 'node:test';
import assert from 'node:assert/strict';
import { paintTextLayout } from '../src/lib/mse/text-painter.mjs';
const run = { text: 'AB\n', characterStart: 0, font: {}, soft: false };
const layout = { scale: 0.5, measurement: { runs: [run] }, lines: [{ start: 0, end: 3, positions: [5,10,17,17], top: 8, justifying: false }] };
function setup() {
  const calls=[];
  return { calls, ctx: { save:()=>calls.push('save'),restore:()=>calls.push('restore'),translate:(x,y)=>calls.push([x,y]) }, fonts:{prepare:async()=>({draw:(ctx,text,x,y,scale)=>calls.push({text,x,y,scale})})} };
}
test('text painter preserves grouped text, but justified lines draw separate characters', async () => {
  const a=setup(); await paintTextLayout(a.ctx,layout,a.fonts);
  assert.deepEqual(a.calls[2],{text:'AB',x:5,y:8,scale:0.5});
  assert.equal(a.calls.at(-1),'restore');
  const b=setup(); await paintTextLayout(b.ctx,{...layout,lines:[{...layout.lines[0],justifying:true}]},b.fonts);
  assert.deepEqual(b.calls.slice(2,-1),[{text:'A',x:5,y:8,scale:0.5},{text:'B',x:10,y:8,scale:0.5}]);
});
test('font preparation failures happen before changing the canvas', async () => {
  const a=setup();
  const prepare=a.fonts.prepare;
  a.fonts.prepare=async font=>{if(font.name==='Missing')throw Object.assign(new Error('Missing font'),{code:'font-missing'});return prepare(font);};
  await assert.rejects(()=>paintTextLayout(a.ctx,{...layout,measurement:{runs:[run,{...run,font:{name:'Missing'}}]}},a.fonts),{code:'font-missing'});
  assert.deepEqual(a.calls,[]);
});

test('field stretch scales the prepared text drawing without changing layout coordinates',async()=>{
  const a=setup();a.ctx.scale=(x,y)=>a.calls.push({stretch:[x,y]});
  await paintTextLayout(a.ctx,{...layout,stretch:0.6},a.fonts,{x:7,y:9});
  assert.deepEqual(a.calls.slice(1,4),[[7,9],{stretch:[0.6,1]},{text:'AB',x:5,y:8,scale:0.5}]);
  assert.equal(layout.lines[0].positions[0],5);
});

test('native line visibility skips wholly outside lines but draws intersecting lines without clipping',async()=>{
  const a=setup(),positions=[0,5];
  const runs=['A','B','C','D'].map((text,i)=>({text,characterStart:i,font:i===0||i===3?{name:'Unavailable'}:{},soft:false}));
  a.fonts.prepare=async font=>{assert.notEqual(font.name,'Unavailable');return {draw:(_ctx,text,x,y)=>a.calls.push({text,x,y})};};
  await paintTextLayout(a.ctx,{scale:1,measurement:{runs},lines:[
    {start:0,end:1,positions,top:-10,height:10},
    {start:1,end:2,positions,top:-5,height:10},
    {start:2,end:3,positions,top:15,height:10},
    {start:3,end:4,positions,top:20,height:10},
  ]},a.fonts,{height:20});
  assert.deepEqual(a.calls.filter(v=>v?.text),[{text:'B',x:0,y:-5},{text:'C',x:0,y:15}]);
});
