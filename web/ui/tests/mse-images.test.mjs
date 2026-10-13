import test from 'node:test';
import assert from 'node:assert/strict';
import { MseRuntime } from '../src/lib/mse/script-runtime.mjs';
import { coreBuiltins } from '../src/lib/mse/script-builtins.mjs';
import { imageBuiltins } from '../src/lib/mse/image-builtins.mjs';
import { MseImageRenderer } from '../src/lib/mse/image-renderer.mjs';

const vm = () => new MseRuntime({ builtins: { ...coreBuiltins(), ...imageBuiltins('example.mse-style') } });
const raster = (width, values, hasAlpha = true) => ({ width, height: values.length / 4 / width, data: new Uint8ClampedArray(values), hasAlpha });

test('enlargement pads alpha, adjusts generated dimensions, and composes with shadows', async () => {
  const source = raster(2, [100,50,25,255, 70,80,90,128]);
  const renderer = new MseImageRenderer({loadAsset:async()=>source});
  const runtime=vm();
  const result=await renderer.render(runtime.run('enlarge_image("frame.png",border_size:1)'));
  assert.equal(result.width,6); assert.equal(result.height,3);
  assert.deepEqual([...result.data.slice(32,40)],[100,50,25,255,70,80,90,128]);
  assert.equal(result.data[3],0);
  const blank=await renderer.render(runtime.run('enlarge(nil,border_size:0.25)'),{width:20,height:12});
  assert.equal(blank.width,14); assert.equal(blank.height,8);
  const shadow=await renderer.render(runtime.run('drop_shadow(enlarge("frame.png",border_size:1),offset_x:0.2,offset_y:0,alpha:1,blur_radius:0.2,color:rgb(0,0,0))'));
  assert.equal(shadow.width,6); assert.equal(shadow.height,3);
  assert.ok(shadow.data.some((v,i)=>i%4===3 && v>result.data[i]));
  assert.deepEqual([...source.data],[100,50,25,255,70,80,90,128]);
  await assert.rejects(renderer.render(runtime.run('enlarge("frame.png",border_size:-0.1)')),{code:'image-enlarge'});
  const opaque=new MseImageRenderer({loadAsset:async()=>({...source,hasAlpha:false})});
  assert.ok((await opaque.render(runtime.run('enlarge("frame.png",border_size:0)'))).data.every((v,i)=>i%4!==3 || v===0));
});

test('drop shadow aliases render clipped Gaussian effects and preserve shared source pixels', async () => {
  const source = raster(3, [200,100,50,0, 200,100,50,255, 200,100,50,0]);
  const renderer = new MseImageRenderer({loadAsset: async()=>source});
  const runtime = vm();
  const args = '"frame.png",offset_x:0,offset_y:0,alpha:1.5,blur_radius:0.5,color:rgb(255,255,255)';
  const result = await renderer.render(runtime.run(`drop_shadow(${args})`));
  const alias = await renderer.render(runtime.run(`add_drop_shadow(${args})`));
  assert.deepEqual(alias,result);
  assert.equal(result.width,3); assert.equal(result.height,1);
  assert.ok(result.data[3]>0);
  assert.deepEqual([...result.data.slice(4,8)],[200,100,50,255]);
  assert.equal(source.data[3],0);
  const unchanged = await renderer.render(runtime.run('drop_shadow("frame.png",alpha:0,blur_radius:0.5)'));
  assert.deepEqual([...unchanged.data],[0,0,0,0,200,100,50,255,0,0,0,0]);
  await assert.rejects(renderer.render(runtime.run('drop_shadow("frame.png",alpha:1,blur_radius:0)')),{code:'image-shadow'});
});

test('stroke scripts preserve source placement, optional compositing, and package ownership', async () => {
  const source = raster(1, [200,100,50,255]);
  const paths = [];
  const renderer = new MseImageRenderer({ loadAsset: async path => { paths.push(path.key); return source; } });
  const runtime = vm();
  const graph = runtime.run('add_stroke_effect("frame.png",color:rgb(10,20,30),radius:1)');
  const included = await renderer.render(graph);
  assert.equal(included.width,3);
  assert.deepEqual([...included.data.slice(16,20)],[200,100,50,255]);
  assert.deepEqual([...included.data.slice(0,4)],[10,20,30,153]);
  const only = await renderer.render(runtime.run('add_stroke_effect("frame.png",color:rgb(10,20,30),radius:1,include_image:false)'));
  assert.deepEqual([...only.data.slice(16,20)],[10,20,30,255]);
  const blurred = await renderer.render(runtime.run('add_stroke_effect("frame.png",color:rgb(10,20,30),radius:1,blur:2)'));
  assert.equal(blurred.width,7);
  assert.deepEqual([...blurred.data.slice(96,100)],[200,100,50,255]);
  assert.deepEqual(paths,Array(3).fill('example.mse-style/frame.png'));
  assert.deepEqual([...source.data],[200,100,50,255]);
});

test('image scripts retain package ownership and recolor source channels without changing alpha', async () => {
  const source = raster(3, [255,0,0,128, 0,255,0,64, 0,0,255,32]);
  const paths = [];
  const renderer = new MseImageRenderer({ loadAsset: async path => { paths.push(path.key); return source; } });
  const runtime = vm();
  const graph = runtime.run('recolor_image("frame.png", red:rgb(10,20,30), green:rgb(40,50,60), blue:rgb(70,80,90))');
  assert.equal(runtime.run('type_name(to_image("frame.png"))'), 'image');
  assert.equal(runtime.run('to_image("frame.png") == to_image("frame.png")'), true);
  assert.equal(runtime.run('to_image("frame.png") == to_image("other.png")'), false);
  assert.deepEqual([...(await renderer.render(graph)).data], [10,20,30,128, 40,50,60,64, 70,80,90,32]);
  assert.deepEqual([...source.data], [255,0,0,128, 0,255,0,64, 0,0,255,32]);
  assert.deepEqual(paths, ['example.mse-style/frame.png']);
  assert.equal(runtime.run('to_image("/other.mse-style/frame.png")').parameters.path.package, 'other.mse-style');
  const dark = await renderer.render(runtime.run('recolor_image("frame.png", color:rgb(10,20,30))'));
  assert.deepEqual([...dark.data], [10,20,30,128, 0,0,0,64, 255,255,255,32]);
});

test('masked blends use each mask color channel and the red channel for alpha', async () => {
  const fixtures = {
    light: raster(1, [200,100,40,200]), dark: raster(1, [20,40,80,100]), mask: raster(1, [255,0,128,255]),
  };
  const renderer = new MseImageRenderer({ loadAsset: async path => fixtures[path.file] });
  const runtime = vm();
  const result = await renderer.render(runtime.run('masked_blend(light:"light", dark:"dark", mask:"mask")'));
  assert.deepEqual([...result.data], [200,40,59,200]);
  const masked = await renderer.render(runtime.run('set_mask(image:"dark", mask:"mask")'));
  assert.deepEqual([...masked.data], [20,40,80,100]);
});

test('linear blends preserve native fixed-point truncation and flips permute whole pixels', async () => {
  const fixtures = { a: raster(2, [255,0,0,200, 255,0,0,200]), b: raster(2, [0,0,255,100, 0,0,255,100]) };
  const renderer = new MseImageRenderer({ loadAsset: async path => fixtures[path.file] });
  const runtime = vm();
  const result = await renderer.render(runtime.run('linear_blend(image1:"a", image2:"b", x1:0, y1:0, x2:1, y2:0)'));
  assert.deepEqual([...result.data], [255,0,0,200, 128,0,127,150]);
  runtime.scope.set('blend', runtime.run('linear_blend(image1:"a", image2:"b", x1:0, y1:0, x2:1, y2:0)'));
  assert.deepEqual([...(await renderer.render(runtime.run('flip_horizontal(blend)'))).data], [128,0,127,150, 255,0,0,200]);
  await assert.rejects(renderer.render(runtime.run('linear_blend(image1:"a", image2:"b", x1:0, y1:0, x2:0, y2:0)')), { code: 'image-blend' });
});

test('fill, invert, opacity, blank images and size failures have explicit raster behavior', async () => {
  const renderer = new MseImageRenderer({ loadAsset: async path => path.file === 'wide' ? raster(2, [0,0,0,255,0,0,0,255]) : raster(1, [10,20,30,101]) });
  const runtime = vm();
  assert.deepEqual([...(await renderer.render(runtime.run('set_alpha(invert_image(fill_image("a", color:rgb(20,40,60))), alpha:0.5)'))).data], [235,215,195,50]);
  const blank = await renderer.render(runtime.run('to_image(nil)'), { width: 2, height: 3 });
  assert.equal(blank.data.length, 24);
  assert.equal(blank.data.every(value => value === 0), true);
  await assert.rejects(renderer.render(runtime.run('masked_blend(light:"a", dark:"wide", mask:"a")')), { code: 'image-size' });
});

test('resizing uses alpha-weighted area samples and mask resizing follows the same rules', async () => {
  const fixtures = {
    transparent: raster(2, [255,0,0,255, 0,0,255,0]),
    opaque: raster(2, [255,0,0,255, 0,0,255,255], false),
    mask: raster(1, [128,128,128,255], false),
  };
  const renderer = new MseImageRenderer({ loadAsset: async path => fixtures[path.file] });
  const runtime = vm();
  assert.deepEqual([...(await renderer.render(runtime.run('resize_image("transparent", width:1, height:1)'))).data], [255,0,0,127]);
  assert.deepEqual([...(await renderer.render(runtime.run('resize_image("opaque", width:3, height:1)'))).data], [255,0,0,255, 127,0,127,255, 0,0,255,255]);
  assert.deepEqual([...(await renderer.render(runtime.run('set_mask(image:"transparent", mask:"mask")'))).data], [255,0,0,128, 0,0,255,0]);
  const minimum = await renderer.render(runtime.run('resize_image("opaque", width:0, height:-1)'));
  assert.deepEqual([minimum.width, minimum.height], [1,1]);
});

test('crop offsets truncate toward zero and uncovered pixels use the supplied background', async () => {
  const renderer = new MseImageRenderer({ loadAsset: async () => raster(2, [10,20,30,255,40,50,60,255], false) });
  const runtime = vm();
  const cropped = await renderer.render(runtime.run('crop("a", width:1, height:1, offset_x:1.9, offset_y:0)'));
  assert.deepEqual([...cropped.data], [40,50,60,255]);
  assert.equal(cropped.hasAlpha, false);
  const expanded = await renderer.render(runtime.run('crop("a", width:3, height:1, offset_x:-1, offset_y:0, background_color:rgba(1,2,3,64))'));
  assert.deepEqual([...expanded.data], [1,2,3,64, 10,20,30,255, 40,50,60,255]);
  assert.equal(expanded.hasAlpha, true);
  await assert.rejects(renderer.render(runtime.run('crop("a", width:0, height:1, offset_x:0, offset_y:0)')), { code: 'image-size' });
});

test('combine modes copy second alpha, preserve first alpha when absent, and normal replaces both', async () => {
  const fixtures = {
    a: raster(1, [100,150,200,40]),
    b: raster(1, [64,128,192,80]),
    opaque: raster(1, [64,128,192,255], false),
  };
  const renderer = new MseImageRenderer({ loadAsset: async path => fixtures[path.file] });
  const runtime = vm();
  const blend = (second, mode) => renderer.render(runtime.run(`combine_blend(image1:"a", image2:"${second}", combine:"${mode}")`));
  assert.deepEqual([...(await blend('b', 'lightmap relative')).data], [50,150,227,80]);
  assert.deepEqual([...(await blend('opaque', 'lightmap relative')).data], [50,150,227,40]);
  const normal = await blend('opaque', 'normal');
  assert.deepEqual([...normal.data], [64,128,192,255]);
  assert.equal(normal.hasAlpha, false);
  assert.deepEqual([...fixtures.a.data], [100,150,200,40]);
  assert.throws(() => runtime.run('combine_blend(image1:"a", image2:"b", combine:"missing")'), { code: 'image-combine' });
});

test('insertion expands for negative offsets, clips when requested, and composes alpha', async () => {
  const fixtures = {
    base: raster(1, [255,0,0,128]), insert: raster(1, [0,0,255,128]),
    opaque: raster(1, [20,40,60,255], false),
  };
  const runtime = vm();
  const renderer = new MseImageRenderer({ loadAsset: async path => fixtures[path.file] });
  const render = options => renderer.render(runtime.run(`insert_image(base_image:"base", inserted_image:"insert", ${options})`));
  assert.deepEqual([...(await render('offset_x:0, offset_y:0')).data], [85,0,170,192]);
  const expanded = await render('offset_x:-1, offset_y:-1');
  assert.equal(expanded.width, 2);
  assert.equal(expanded.height, 2);
  assert.deepEqual([...expanded.data], [0,0,255,128, 0,0,0,0, 0,0,0,0, 255,0,0,128]);
  assert.deepEqual([...(await render('offset_x:-1, offset_y:0, widen:false')).data], [255,0,0,128]);
  const opaque = await renderer.render(runtime.run('insert_image(base_image:"base", inserted_image:"opaque", offset_x:0, offset_y:0)'));
  assert.deepEqual([...opaque.data], [20,40,60,255]);
  assert.deepEqual([...fixtures.base.data], [255,0,0,128]);
  const budgetRenderer = new MseImageRenderer({ loadAsset: async path => fixtures[path.file], maxPixels: 4 });
  await assert.rejects(() => budgetRenderer.render(runtime.run('insert_image(base_image:"base", inserted_image:"insert", offset_x:9, offset_y:9)')), { code: 'image-budget' });
});

test('nine slice preserves corners, stretches center, and clips overlapping borders in native order',async()=>{
  const source=raster(3,Array.from({length:9},(_,i)=>[i*20,0,0,255]).flat());
  const renderer=new MseImageRenderer({loadAsset:async()=>source}),runtime=vm();
  const result=await renderer.render(runtime.run('nine_slice_image("frame",width:5,height:5,left:1,right:1,top:1,bottom:1)'));
  const red=(x,y)=>result.data[(y*5+x)*4];
  assert.deepEqual([red(0,0),red(4,0),red(0,4),red(4,4)],[0,40,120,160]);
  assert.equal(red(2,2),80);assert.equal(red(2,0),20);assert.equal(red(0,2),60);
  const small=await renderer.render(runtime.run('nine_slice_image("frame",width:1,height:1,left:1,right:1,top:1,bottom:1)'));
  assert.deepEqual([...small.data],[160,0,0,255]);
  const full=await renderer.render(runtime.run('nine_slice_image("frame",width:7,height:7,left:3,right:3,top:3,bottom:3)'));
  assert.equal(full.width,7);assert.equal(full.data[(6*7+6)*4],160);
});

test('flip_image selects either axis without modifying cached asset pixels',async()=>{
  const source=raster(2,[1,0,0,255,2,0,0,128,3,0,0,64,4,0,0,0]);
  const renderer=new MseImageRenderer({loadAsset:async()=>source}),runtime=vm();
  const horizontal=await renderer.render(runtime.run('flip_image("frame.png",horizontal:true)'));
  const vertical=await renderer.render(runtime.run('flip_image("frame.png",horizontal:false)'));
  assert.deepEqual([...horizontal.data].filter((_,i)=>i%4===0),[2,1,4,3]);
  assert.deepEqual([...vertical.data].filter((_,i)=>i%4===0),[3,4,1,2]);
  assert.deepEqual([...vertical.data].filter((_,i)=>i%4===3),[64,0,255,128]);
  assert.deepEqual([...source.data].filter((_,i)=>i%4===0),[1,2,3,4]);
});
