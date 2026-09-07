import test from 'node:test';
import assert from 'node:assert/strict';
import { fullCardImageUrl, materialColor, sectionInk, artBottomRail, reconstructPanel, classifyTitlePanel, classifyTypePanel, printedGlyphHeight, printedTextBounds, detectPanelBounds, detectEnclosedPanelBounds, matchArtBounds, detectPrintedStats, printedStatsTreatment, tracePanelRim, detectStatsPanelBounds } from '../src/lib/card-frame-colors.js';

test('complete text bounds include capitals and descenders without absorbing a frame rail', () => {
  const width=220,height=44,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([230,230,230,255],p*4);
  const rect=(x,y,w,h)=>{for(let row=y;row<y+h;row++)for(let col=x;col<x+w;col++)data.set([15,15,15,255],(row*width+col)*4);};
  rect(0,1,width,2);
  for(const x of [12,28,44,60,76,92])rect(x,15,5,10);
  rect(110,8,8,17);rect(130,15,8,17);
  assert.deepEqual(printedTextBounds({data,width,height}),{x:12,y:8,right:138,bottom:32});
});

test('color reference preserves the displayed printing, face, and cache version', () => {
  assert.equal(fullCardImageUrl('https://cards.scryfall.io/art_crop/back/a/b/id.jpg?123'), 'https://cards.scryfall.io/normal/back/a/b/id.jpg?123');
  assert.equal(fullCardImageUrl('https://custom.example/art.jpg'), '');
});

test('printed glyphs do not replace the dominant panel material', () => {
  const data = new Uint8ClampedArray(400 * 4);
  for (let i = 0; i < 400; i++) data.set(i < 150 ? [5, 5, 5, 255] : [230, 218, 190, 255], i * 4);
  assert.deepEqual(materialColor(data), [230, 218, 190]);
});

test('ink sampling handles light and dark printed lettering', () => {
  for (const [paper, ink] of [[[40, 45, 55], [240, 235, 220]], [[235, 230, 205], [15, 20, 25]]]) {
    const width = 100, height = 30, data = new Uint8ClampedArray(width * height * 4);
    for (let i = 0; i < width * height; i++) data.set([...paper, 255], i * 4);
    for (const left of [20, 40, 60]) for (let y = 8; y < 22; y++) for (let x = left; x < left + 4; x++) data.set([...ink, 255], (y * width + x) * 4);
    assert.deepEqual(sectionInk({width, height, data}), ink[0] > paper[0] ? [255, 255, 255] : [0, 0, 0]);
  }
});





test('panel reconstruction removes light and dark print while preserving clean texture', () => {
  for (const [paper, ink] of [[65, 245], [225, 15]]) {
    const width = 180, height = 40, data = new Uint8ClampedArray(width * height * 4);
    for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
      const grain = (x * 7 + y * 3) % 9;
      const value = x < 110 && x % 18 < 4 && y > 12 && y < 27 ? ink : paper + grain;
      data.set([value, value, value, 255], (y * width + x) * 4);
    }
    const panel = reconstructPanel({ data, width, height });
    assert.ok(panel);
    assert.equal(panel.width, width);
    assert.equal(panel.height, height);
    for (let p = 0; p < width * height; p++) {
      assert.ok(panel.data[p * 4] >= paper && panel.data[p * 4] <= paper + 8, 'printed ink is removed');
      if (!panel.mask[p]) assert.equal(panel.data[p * 4], data[p * 4], 'original clean texture is unchanged');
    }
    assert.deepEqual(reconstructPanel({data, width, height}).data, panel.data, 'stable between renders');
  }
});


test('bottom rail detection requires a boundary across most of the art', () => {
  const width = 240, height = 180;
  const make = framed => {
    const data = new Uint8ClampedArray(width * height * 4);
    for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
      const value = y >= height - 6 && (framed || x < width / 3) ? 50 : 180;
      data.set([value, value, value, 255], (y * width + x) * 4);
    }
    return {data, width, height};
  };
  assert.equal(artBottomRail(make(false)), 0);
  assert.ok(artBottomRail(make(true)) >= 5);
});


test('title classifier requires enclosing strokes on both sides, regardless of material color', () => {
  const width = 488, height = 680;
  for (const paper of [65, 205]) {
    const make = sides => {
      const data = new Uint8ClampedArray(width * height * 4);
      for (let p = 0; p < width * height; p++) data.set([paper, paper, paper, 255], p * 4);
      const ink = paper < 100 ? 245 : 20;
      for (const x of sides) for (let y = 35; y < 65; y++) data.set([ink, ink, ink, 255], (y * width + x) * 4);
      return {data, width, height};
    };
    assert.equal(classifyTitlePanel(make([])).kind, 'integrated');
    assert.equal(classifyTitlePanel(make([32])).kind, 'integrated');
    const result = classifyTitlePanel(make([32, width - 33]));
    assert.equal(result.kind, 'panel');
    assert.deepEqual(result.border, Array(3).fill(paper < 100 ? 245 : 20));
  }
});


test('type enclosure is classified independently of the title', () => {
  const width = 488, height = 680, data = new Uint8ClampedArray(width * height * 4);
  for (let p = 0; p < width * height; p++) data.set([210, 210, 210, 255], p * 4);
  for (const x of [32, width - 33]) for (let y = 382; y < 417; y++) data.set([20, 20, 20, 255], (y * width + x) * 4);
  const scan = {data, width, height};
  assert.equal(classifyTitlePanel(scan).kind, 'integrated');
  assert.equal(classifyTypePanel(scan).kind, 'panel');
  assert.deepEqual(classifyTypePanel(scan).border, [20, 20, 20]);
});


test('printed glyph sizing uses repeated letters and ignores borders and punctuation', () => {
  for (const [paper, ink] of [[230, 20], [35, 240]]) {
    const width = 160, height = 40, data = new Uint8ClampedArray(width * height * 4);
    for (let p = 0; p < width * height; p++) data.set([paper, paper, paper, 255], p * 4);
    const mark = (x, y) => data.set([ink, ink, ink, 255], (y * width + x) * 4);
    for (let x = 0; x < width; x++) mark(x, 1);
    assert.equal(printedGlyphHeight({data, width, height}), null);
    for (const left of [12, 32, 52, 72, 92, 112]) {
      for (let y = 12; y < 24; y++) for (let x = left; x < left + 4; x++) mark(x, y);
      mark(left + 7, 28);
    }
    assert.equal(printedGlyphHeight({data, width, height}), 12);
  }
});


test('whole-panel bounds reject blank scans and locate an enclosed title', () => {
  const width = 488, height = 680, data = new Uint8ClampedArray(width * height * 4);
  for (let p = 0; p < width * height; p++) data.set([65,65,65,255],p*4);
  assert.equal(detectPanelBounds({data,width,height}, 'title'), null);
  for (let y = 32; y <= 72; y++) for (let x = 32; x <= 455; x++) data.set([220,220,220,255],(y*width+x)*4);
  const bounds = detectPanelBounds({data,width,height}, 'title');
  assert.ok(bounds);
  assert.ok(Math.abs(bounds.x-32) <= 4);
  assert.ok(Math.abs(bounds.y-32) <= 4);
  assert.ok(bounds.width > 415 && bounds.width < 435);
  assert.equal(detectPanelBounds({data,width,height}, 'rules'), null);
});


test('enclosure detection keeps a complete panel and rejects an adjoining art edge', () => {
  const width=488,height=680,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++) data.set([220,220,215,255],p*4);
  const stroke=(x,y)=>data.set([20,20,20,255],(y*width+x)*4);
  for(let x=35;x<=453;x++) {stroke(x,29);stroke(x,70);}
  for(let y=29;y<=70;y++) {stroke(35,y);stroke(453,y);}
  for(let x=60;x<160;x+=12) for(let y=40;y<52;y++) stroke(x,y);
  const bounds=detectEnclosedPanelBounds({data,width,height},'title');
  assert.deepEqual(bounds,{x:34,y:28,width:421,height:44});
  // A contour running into the artwork cannot establish an isolated panel.
  for(let y=70;y<90;y++) stroke(453,y);
  assert.equal(detectEnclosedPanelBounds({data,width,height},'title'),null);
});


test('rules reconstruction masks faint separators without flattening clean paper', () => {
  const width=100,height=40,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++) data.set([235,240,245,255],p*4);
  for(let x=10;x<90;x++) data.set([225,230,235,255],(20*width+x)*4);
  const cleaned=reconstructPanel({data,width,height},{removeSeparators:true});
  assert.ok(cleaned);
  assert.deepEqual(Array.from(cleaned.data),Array.from(new Uint8ClampedArray(Array.from({length:width*height},()=>[235,240,245,255]).flat())));
});








test('P/T location follows aligned light or dark glyphs and ignores empty scans', () => {
  const width=488,height=680;
  for(const white of [false,true]) {
    const data=new Uint8ClampedArray(width*height*4);
    for(let p=0;p<width*height;p++) data.set(white?[35,30,25,255]:[235,230,220,255],p*4);
    assert.equal(detectPrintedStats({data,width,height}),null);
    for(const x0 of [402,414,425]) for(let y=613;y<634;y++) for(let x=x0;x<x0+6;x++) data.set(white?[240,240,235,255]:[20,20,20,255],(y*width+x)*4);
    const box=detectPrintedStats({data,width,height});
    assert.ok(box);
    assert.equal(box.x,402);assert.equal(box.y,613);assert.equal(box.height,21);
    assert.equal(printedStatsTreatment({data,width,height},box),'text');
    const edge=(x,y)=>data.set(white?[240,240,235,255]:[20,20,20,255],(y*width+x)*4);
    for(let x=390;x<=442;x++){edge(x,607);edge(x,640);}
    for(let y=607;y<=640;y++){edge(390,y);edge(442,y);}
    assert.equal(printedStatsTreatment({data,width,height},box),'panel');

  }
});

test('art matching finds a crop independently of the card frame color', () => {
  const width=200,height=280,aw=80,ah=60;
  const art=new Uint8ClampedArray(aw*ah*4),data=new Uint8ClampedArray(width*height*4);
  for(let y=0;y<ah;y++) for(let x=0;x<aw;x++) art.set([130+90*Math.sin(x*.21),130+90*Math.cos(y*.31),130+90*Math.sin((x+y)*.18),255],(y*aw+x)*4);
  for(let p=0;p<width*height;p++) data.set([35,30,25,255],p*4);
  for(let y=30;y<156;y++) for(let x=16;x<184;x++) {
    const a=(Math.floor((y-30)*ah/126)*aw+Math.floor((x-16)*aw/168))*4;
    data.set(art.subarray(a,a+4),(y*width+x)*4);
  }
  const bounds=matchArtBounds({data,width,height},{data:art,width:aw,height:ah});
  assert.ok(bounds);assert.ok(Math.abs(bounds.x-16)<=3);assert.ok(Math.abs(bounds.width-168)<=4);assert.ok(Math.abs(bounds.y-30)<=3);
});


test('vector rim keeps asymmetric source colors while excluding central print', () => {
  const width=80,height=40,data=new Uint8ClampedArray(width*height*4);
  for(let y=0;y<height;y++)for(let x=0;x<width;x++) {
    const rim=x<6||x>=width-6||y<6||y>=height-6;
    data.set(rim?(x<width/2?[32,40,48,255]:[200,208,216,255]):[255,0,255,255],(y*width+x)*4);
  }
  const svg=tracePanelRim({data,width,height},{x:0,y:0,width,height},'rules');
  assert.match(svg,/rgb\(32,40,48\)/);
  assert.match(svg,/rgb\(200,208,216\)/);
  assert.doesNotMatch(svg,/rgb\(255,0,255\)/);
  assert.equal(detectStatsPanelBounds({data,width,height},null),null);
});


test('lower rail detection includes the full stroke beyond its first transition', () => {
  const width=488,height=680,data=new Uint8ClampedArray(width*height*4).fill(220);
  const ink=(x,y)=>data.set([30,30,30,255],(y*width+x)*4);
  for(let y=32;y<=72;y++)for(let x=30;x<=455;x++)
    if(y===32||y>=69||x===30||x===455)ink(x,y);
  const box=detectPanelBounds({data,width,height},'title');
  assert.ok(box.y+box.height>72,'all four pixels of the lower stroke remain inside the crop');
});
