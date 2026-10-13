import test from 'node:test';
import assert from 'node:assert/strict';
import {symbolDrawOrder} from '../src/lib/mse/symbol-draw-order.mjs';
const shape=name=>({type:'shape',name});
test('rotational copies follow native draw order and overlap boundaries',()=>{
  const copies=symbolDrawOrder({parts:[{type:'symmetry',kind:'rotation',copies:4,center:[0.5,0.5],handle:[1,0],parts:[shape('top'),shape('bottom')]}]});
  assert.deepEqual(copies.map(p=>p.shape.name),['bottom','bottom','bottom','bottom','top','top','top','top']);
  assert.deepEqual(copies.map(p=>p.allowOverlap),[true,false,false,false,true,false,false,false]);
  const points=copies.slice(0,4).map(p=>p.transform([0.75,0.5]).map(v=>Math.round(v*100)));
  assert.deepEqual(points,[[50,25],[25,50],[50,75],[75,50]]);
});
test('reflections compose with parent transforms and drop the unmatched odd copy',()=>{
  const copies=symbolDrawOrder({parts:[{type:'symmetry',kind:'reflection',copies:3,center:[0,0],handle:[1,0],parts:[shape('child')]}]});
  assert.equal(copies.length,2);
  const p=copies[0].transform([2,3]);assert.ok(Math.abs(p[0]+2)<1e-10);assert.ok(Math.abs(p[1]-3)<1e-10);
  assert.deepEqual(copies[1].transform([2,3]),[2,3]);
  assert.throws(()=>symbolDrawOrder({parts:[{type:'symmetry',kind:'rotation',copies:100,center:[0,0],handle:[1,0],parts:[shape('child')]}]},{maxShapes:10}),{code:'symbol-budget'});
});
