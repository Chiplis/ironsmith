import test from 'node:test';
import assert from 'node:assert/strict';
import {decorateTextRaster,prepareFontDrawing} from '../src/lib/mse/font-painter.mjs';
const mask={width:1,height:1,data:new Uint8ClampedArray([255,255,255,255]),hasAlpha:true};
test('text effects compose the glyph inside its stroke and avoid double painting equal colors',()=>{
  const result=decorateTextRaster(mask,[255,0,0,128],[0,0,255,255],1,0);
  assert.equal(result.margin,1);assert.equal(result.raster.width,3);
  assert.deepEqual([...result.raster.data.slice(16,20)],[128,0,127,255]);
  const equal=decorateTextRaster(mask,[10,20,30,128],[10,20,30,128],1,0);
  assert.equal(equal.raster.data[19],128);
  assert.equal(decorateTextRaster(mask,[1,2,3,0],[0,0,0,255],1,0),null);
  assert.deepEqual([...mask.data],[255,255,255,255]);
});
test('fonts without bitmap effects preserve the provider including line decorations',async()=>{
  const provider={draw(){}};
  assert.equal(await prepareFontDrawing({}, {prepare:async()=>provider}),provider);
  assert.equal(await prepareFontDrawing({underline:true,strikethrough:true},{prepare:async()=>provider}),provider);
});

test('ordinary native text rasterizes at fourfold resolution with subpixel-aware caching',async()=>{
  const rasterCalls=[],blits=[];
  const provider={rasterScale:4,measure:()=>({width:2,height:3}),
    drawRaster:(ctx,text,x,y,scale)=>rasterCalls.push({text,x,y,scale}),draw(){throw new Error('Unscaled draw');}};
  const createCanvas=(width,height)=>({width,height,getContext:()=>({
    getImageData:()=>({data:new Uint8ClampedArray(width*height*4)}),
    createImageData:(w,h)=>({data:new Uint8ClampedArray(w*h*4)}),putImageData(){},
  })});
  const font={size:12,color:{channels:[0,0,0,255]}};
  const painter=await prepareFontDrawing(font,{prepare:async()=>provider},{createCanvas});
  const target={drawImage:(...args)=>blits.push(args)};
  painter.draw(target,'A',2.5,3.75,0.9);
  painter.draw(target,'A',8.5,7.75,0.9);
  painter.draw(target,'A',2.75,3.75,0.9);
  assert.deepEqual(rasterCalls,[{text:'A',x:2,y:3,scale:0.9},{text:'A',x:3,y:3,scale:0.9}]);
  assert.deepEqual(blits.map(([,x,y])=>[x,y]),[[2,3],[8,7],[2,3]]);
  assert.equal(blits[0][0],blits[1][0]);assert.notEqual(blits[0][0],blits[2][0]);
});
