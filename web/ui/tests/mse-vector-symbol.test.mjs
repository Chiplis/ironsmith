import test from 'node:test';
import assert from 'node:assert/strict';
import {parseVectorSymbol,defaultVectorSymbol} from '../src/lib/mse/vector-symbol.mjs';
test('symbol records retain repeated shapes, cubic handles and symmetry structure',()=>{
  const symbol=parseVectorSymbol('mse version: 0.3.6\npart:\n\ttype: symmetry\n\tkind: reflection\n\tcopies: 3\n\tcenter: (0.5,0.5)\n\tpart:\n\t\tcombine: subtract\n\t\tpoint:\n\t\t\tposition: (0.2,0.3)\n\t\t\tline after: curve\n\t\t\thandle after: (0.1,-0.2)\n\t\tpoint:\n\t\t\tposition: (0.8,0.9)\n');
  assert.equal(symbol.parts[0].copies,3);
  const shape=symbol.parts[0].parts[0];assert.equal(shape.combine,'subtract');
  assert.deepEqual(shape.points[0].after,[0.1,-0.2]);assert.equal(shape.points[1].segment,'line');
  assert.equal(defaultVectorSymbol().parts[0].points.length,4);
  assert.throws(()=>parseVectorSymbol('part:\n\tpoint:\n\t\tposition: (NaN,0)'),{code:'symbol-vector'});
});

import {constrainSymbolShape,symbolShapePolygon} from '../src/lib/mse/symbol-geometry.mjs';
test('symbol geometry enforces handle locks without changing parsed data',()=>{
  const shape=defaultVectorSymbol().parts[0];
  shape.points[0].lock='size';shape.points[0].before=[2,4];shape.points[0].after=[0,2];
  const prepared=constrainSymbolShape(shape);
  assert.deepEqual(prepared.points[0].before,[1,1]);assert.deepEqual(prepared.points[0].after,[-1,-1]);
  assert.deepEqual(shape.points[0].before,[2,4]);
  assert.deepEqual(symbolShapePolygon(shape),shape.points.map(p=>p.position));
});
test('curve tessellation retains MSE midpoint sampling and applies transforms after subdivision',()=>{
  const shape={type:'shape',points:[{position:[0,0],before:[0,0],after:[0,1],segment:'curve',lock:'free'},{position:[1,0],before:[0,1],after:[0,0],segment:'line',lock:'free'}]};
  const polygon=symbolShapePolygon(shape);
  assert.ok(polygon.length>2);assert.ok(polygon.some(p=>p[0]===0.5&&p[1]===0.75));
  assert.deepEqual(polygon.at(-1),[1,0]);
  assert.deepEqual(symbolShapePolygon(shape,{transform:p=>p.map(x=>x*100)}),polygon.map(p=>p.map(x=>x*100)));
});

import {symbolShapeBounds,symbolBounds,symbolAspectRatio} from '../src/lib/mse/symbol-geometry.mjs';
test('symbol bounds include curve extrema rather than just endpoints or control handles',()=>{
  const shape={type:'shape',points:[{position:[0,0],before:[0,0],after:[0,1],segment:'curve',lock:'free'},{position:[1,0],before:[0,1],after:[0,0],segment:'line',lock:'free'}]};
  assert.deepEqual(symbolShapeBounds(shape),{min:[0,0],max:[1,0.75]});
  assert.deepEqual(symbolShapeBounds(shape,{transform:([x,y])=>[y+2,x-3]}),{min:[2,-3],max:[2.75,-2]});
  assert.equal(symbolAspectRatio(symbolBounds(defaultVectorSymbol())),1);
  assert.equal(symbolAspectRatio({min:[0,0.25],max:[1,0.75]}),2);
});
test('symbol bounds account for nested symmetry copies',()=>{
  const shape=defaultVectorSymbol().parts[0];shape.points=shape.points.map(p=>({...p,position:p.position.map(v=>v/4)}));
  const bounds=symbolBounds({parts:[{type:'symmetry',kind:'rotation',copies:2,center:[0.5,0.5],handle:[1,0],parts:[shape]}]});
  assert.ok(Math.abs(bounds.min[0]-0.05)<1e-12);assert.ok(Math.abs(bounds.max[0]-0.95)<1e-12);
  assert.ok(Math.abs(bounds.max[1]-0.95)<1e-12);
});

import {prepareVectorSymbol} from '../src/lib/mse/symbol-geometry.mjs';
test('legacy symbol preparation converts coordinates and constrained handles without changing input',()=>{
  const shape=defaultVectorSymbol().parts[0];
  shape.name='';
  shape.points=shape.points.map(p=>({...p,position:p.position.map(v=>v*500)}));
  shape.points[0]={...shape.points[0],lock:'size',before:[50,100],after:[-150,0]};
  const symbol={version:'',parts:[{type:'group',parts:[shape]}]},before=structuredClone(symbol);
  const prepared=prepareVectorSymbol(symbol).parts[0].parts[0];
  assert.deepEqual(prepared.points.map(p=>p.position),defaultVectorSymbol().parts[0].points.map(p=>p.position));
  assert.deepEqual(prepared.points[0].before,[0.2,0.1]);
  assert.deepEqual(prepared.points[0].after,[-0.2,-0.1]);
  assert.equal(prepared.name,'Shape');assert.deepEqual(symbol,before);
});
test('legacy coordinate detection uses both actual curve bounds and applies independently to each shape',()=>{
  const point=(position,after=[0,0],before=[0,0],segment='line')=>({position,after,before,segment,lock:'free'});
  // Endpoints stay below 100 vertically, but this curve reaches y=150.
  const curve={type:'shape',name:'Curve',points:[point([0,0],[0,200],[0,0],'curve'),point([500,0],[0,0],[0,200])]};
  const thin={type:'shape',name:'Thin',points:[point([0,0]),point([500,99])]};
  const small=defaultVectorSymbol().parts[0];
  for(const version of ['', '0.0.0','0-0-0']){
    const prepared=prepareVectorSymbol({version,parts:[curve,thin,small]});
    assert.deepEqual(symbolShapeBounds(prepared.parts[0]),{min:[0,0],max:[1,0.30000000000000004]});
    assert.deepEqual(prepared.parts[1],thin);assert.deepEqual(prepared.parts[2],small);
  }
  const modern=prepareVectorSymbol({version:'0.3.5',parts:[curve]});
  assert.deepEqual(modern.parts[0],curve);
});
