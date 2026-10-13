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
