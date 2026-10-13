import test from 'node:test';
import assert from 'node:assert/strict';
import {paintMseCard} from '../src/lib/mse/card-painter.mjs';

function setup(fields){
  const surfaces=[],commits=[];
  const createCanvas=(width,height)=>{
    const calls=[];
    const dc={calls,save(){},restore(){},translate(...v){calls.push(['translate',...v]);},rotate(v){calls.push(['rotate',v]);},
      fillRect(){},beginPath(){},rect(...v){calls.push(['rect',...v]);},clip(v){calls.push(['clip',v]);},
      roundRect(...v){calls.push(['roundRect',...v]);},fill(){},
      createImageData(w,h){return {data:new Uint8ClampedArray(w*h*4)};},
      putImageData(p){calls.push(['pixels',[...p.data]]);},drawImage(...v){calls.push(['drawImage',...v]);}};
    const canvas={width,height,getContext:()=>dc};surfaces.push(canvas);return canvas;
  };
  const context={layoutRevision:0,prepareTextLayouts:async()=>{},
    template:{name:'test.mse-style',style:{text:(key,fallback)=>({'card width':'10','card height':'10'}[key]??fallback)},
      layouts:new Map(fields.map(f=>[f.name,{programs:new Map(Object.keys(f.props||{}).map(k=>[k,true]))}])),extraLayouts:new Map()},
    cells:{card:new Map(fields.map(f=>[f.name,{definition:{record:{text:()=>f.type}}}]))},
    fieldVisible:()=>true,field:(group,name)=>fields.find(f=>f.name===name).value,
    property:(name,key)=>fields.find(f=>f.name===name).props?.[key]??({left:0,top:0,width:2,height:1,angle:0}[key]),
    textMeasurements:new Map()};
  return {context,surfaces,commits,createCanvas,target:{drawImage:(...v)=>commits.push(v)},renderer:{maxPixels:1000}};
}
test('masked color uses mask red channel independently of color alpha and commits once',async()=>{
  const s=setup([{name:'border',type:'color',value:'rgba(10,20,30,128)',props:{mask:'mask.png'}}]);
  s.renderer.render=async()=>({width:2,height:1,hasAlpha:true,data:new Uint8ClampedArray([255,0,0,0,128,255,255,255])});
  await paintMseCard(s.target,s.context,{},s.renderer,s);
  assert.deepEqual(s.surfaces[1].getContext().calls[0],['pixels',[10,20,30,255,10,20,30,128]]);
  assert.equal(s.commits.length,1);
});
test('unmasked color draws a hollow rounded border with native default widths',async()=>{
  const s=setup([{name:'border',type:'color',value:'red',props:{width:8,height:6,left_width:1,right_width:2,top_width:1,bottom_width:2,radius:2}}]);
  await paintMseCard(s.target,s.context,{},s.renderer,s);
  assert.deepEqual(s.surfaces[0].getContext().calls.slice(1),[['rect',0,0,8,6],['rect',1,1,5,3],['clip','evenodd'],['roundRect',0,0,8,6,2]]);
});
test('a later field failure leaves the destination untouched',async()=>{
  const s=setup([{name:'first',type:'color',value:'red'},{name:'art',type:'image',value:'art.png'}]);
  s.loadCardImage=async()=>{throw new Error('missing artwork');};
  await assert.rejects(()=>paintMseCard(s.target,s.context,{},s.renderer,s),{message:'missing artwork',field:'card.art'});
  assert.equal(s.commits.length,0);
});
test('an edit during image loading prevents stale card commitment',async()=>{
  const s=setup([{name:'art',type:'image',value:'art.png'}]);
  s.loadCardImage=async()=>{s.context.layoutRevision++;return null;};
  await assert.rejects(()=>paintMseCard(s.target,s.context,{},s.renderer,s),{code:'layout-stale'});
  assert.equal(s.commits.length,0);
});

test('field rotation follows MSE counterclockwise angles about the field origin',async()=>{
  for(const angle of [90,180,270,-90,450]){
    const s=setup([{name:'rotated',type:'color',value:'red',props:{left:8,top:2,angle}}]);
    await paintMseCard(s.target,s.context,{},s.renderer,s);
    assert.deepEqual(s.surfaces[0].getContext().calls.slice(0,2),[['translate',8,2],['rotate',-(angle%360)*Math.PI/180]]);
    assert.equal(s.commits.length,1);
  }
  const s=setup([{name:'invalid',type:'color',value:'red',props:{angle:Infinity}}]);
  await assert.rejects(()=>paintMseCard(s.target,s.context,{},s.renderer,s),{code:'card-rotation'});
  assert.equal(s.commits.length,0);
});

test('artwork is resized to the field and masked without changing the source',async()=>{
  const s=setup([{name:'art',type:'image',value:'art.png',props:{width:2,height:1,mask:'mask.png'}}]);
  const source={width:1,height:1,hasAlpha:true,data:new Uint8ClampedArray([10,20,30,128])};
  s.loadCardImage=async()=>source;
  s.renderer.render=async()=>({width:2,height:1,hasAlpha:false,data:new Uint8ClampedArray([255,0,0,255,128,0,0,255])});
  await paintMseCard(s.target,s.context,{},s.renderer,s);
  assert.deepEqual(s.surfaces[1].getContext().calls[0],['pixels',[10,20,30,128,10,20,30,64]]);
  assert.deepEqual([...source.data],[10,20,30,128]);
});
test('empty artwork fields use the template default image',async()=>{
  const s=setup([{name:'art',type:'image',value:'',props:{default:'fallback.png'}}]);
  s.renderer.render=async graph=>{assert.equal(graph.parameters.path.file,'fallback.png');return {width:1,height:1,hasAlpha:false,data:new Uint8ClampedArray([40,50,60,255])};};
  await paintMseCard(s.target,s.context,{},s.renderer,s);
  assert.deepEqual(s.surfaces[1].getContext().calls[0],['pixels',[40,50,60,255,40,50,60,255]]);
});

test('missing artwork uses native checker cells, clips partial checks and applies image masks',async()=>{
  const s=setup([{name:'art',type:'image',value:null,props:{width:41,height:13,mask:'mask.png',angle:90}}]);
  s.renderer.render=async()=>{
    const data=new Uint8ClampedArray(41*13*4).fill(255);
    for(let i=0;i<data.length;i+=4)data[i]=128;
    return {width:41,height:13,hasAlpha:false,data};
  };
  await paintMseCard(s.target,s.context,{},s.renderer,s);
  const pixels=s.surfaces[1].getContext().calls.find(call=>call[0]==='pixels')[1];
  const sample=(x,y)=>pixels.slice((y*41+x)*4,(y*41+x)*4+4);
  assert.deepEqual(sample(0,0),[235,235,235,128]);
  assert.deepEqual(sample(10,0),[255,255,255,128]);
  assert.deepEqual(sample(10,10),[235,235,235,128]);
  assert.deepEqual(sample(40,12),[255,255,255,128]);
  assert(s.surfaces[0].getContext().calls.some(call=>call[0]==='rotate'&&call[1]===-Math.PI/2));
  const narrow=setup([{name:'art',type:'image',value:null,props:{width:40,height:13}}]);
  await paintMseCard(narrow.target,narrow.context,{},narrow.renderer,narrow);
  assert.equal(narrow.surfaces.length,1);
});
