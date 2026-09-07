import test from 'node:test';
import assert from 'node:assert/strict';
import {detectInnerFrameBorder, sampleInnerFrameBevel, rasterizeFrameBevel, detectEmbeddedArtFrame} from '../src/lib/card-border-analysis.js';

function printing({color = [16, 17, 18], paper = [160, 130, 90], thickness = 22, footer = false} = {}) {
  const width = 488, height = 680, data = new Uint8ClampedArray(width * height * 4);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const outside = Math.min(x, y, width - 1 - x, height - 1 - y) < thickness;
    const rgb = outside || (footer && y > height - 65) ? color : paper.map(v => v + (x * 3 + y * 7) % 5);
    data.set([...rgb, 255], (y * width + x) * 4);
  }
  return {data, width, height};
}

test('locates the material inside black or white card borders and preserves their bounds and colors', () => {
  for (const color of [[16, 17, 18], [238, 238, 233]]) {
    const border = detectInnerFrameBorder(printing({color}));
    assert.equal(border.kind, 'closed');
    assert.deepEqual(border.colors.left, color);
    assert.equal(border.outerRadius, null, 'opaque matching corners are ambiguous');
    assert.equal(border.innerRadius, 0);
    assert.deepEqual(border.bounds, {x:22, y:22, width:444, height:636});
    assert.ok(Object.values(border.edges).every(Boolean));
    assert.ok(border.confidence >= .9);
  }
});

test('recognizes coherent low-contrast edges on a dark frame', () => {
  const border = detectInnerFrameBorder(printing({paper: [26, 29, 31]}));
  assert.equal(border.kind, 'closed');
  assert.ok(Math.abs(border.bounds.x - 22) <= 2);
});

test('leaves an unsupported lower edge transparent when the frame merges into a collector strip', () => {
  const scan = printing({footer: true}), border = detectInnerFrameBorder(scan);
  assert.equal(border.kind, 'partial');
  assert.equal(border.edges.bottom, false);
  assert.equal(border.edges.top, true);
  const rim = sampleInnerFrameBevel(scan, border);
  assert.ok(rim.profiles.bottom.every(value=>value===0));
});

test('does not mistake a flat scan or two isolated frame bars for an enclosure', () => {
  assert.equal(detectInnerFrameBorder(printing({paper: [16, 17, 18]})), null);
  const scan = printing();
  for (let y = 40; y < scan.height - 40; y++) for (let x = 0; x < scan.width; x++) {
    if (x > 30 && x < scan.width - 30) continue;
    scan.data.set([50 + (y * 13) % 150, 80 + (y * 7) % 100, 100, 255], (y * scan.width + x) * 4);
  }
  assert.equal(detectInnerFrameBorder(scan), null);
  assert.equal(sampleInnerFrameBevel(null), null);
});

test('measures side-specific contrast without cloning text or the black exterior', () => {
  const scan = printing({paper:[150,150,150]}), border = detectInnerFrameBorder(scan);
  for (let d=0;d<6;d++) {
    const dark=60+d*15, bright=225-d*12;
    for (let y=22;y<658;y++) {
      scan.data.set([dark,dark,dark,255],(y*scan.width+22+d)*4);
      scan.data.set([bright,bright,bright,255],(y*scan.width+465-d)*4);
    }
    for (let x=22;x<466;x++) scan.data.set([bright,bright,bright,255],((22+d)*scan.width+x)*4);
  }
  const before=sampleInnerFrameBevel(scan,border);
  assert.ok(before.profiles.left[0]<-.4,'left shadow is retained');
  assert.ok(before.profiles.right[0]>.5,'right highlight has independent polarity');
  assert.ok(before.profiles.top[0]>.5,'top highlight is retained');
  assert.ok(Math.abs(before.profiles.left[7])<.03,'bevel settles into the material');
  // Changes outside the sampled contour or deep in the content cannot create
  // a second outline or contaminate the frame with title glyphs.
  for (let y=34;y<49;y++) for (let x=60;x<140;x++) scan.data.set([1,2,3,255],(y*scan.width+x)*4);
  for (let y=0;y<scan.height;y++) for (let x=0;x<20;x++) scan.data.set([240,240,240,255],(y*scan.width+x)*4);
  assert.deepEqual(sampleInnerFrameBevel(scan,border).profiles,before.profiles);
});

test('a single rounded contour turns the side shadow through the corner with no seams', () => {
  const profiles = {left:Array(8).fill(-.7),top:Array(8).fill(-.7),right:Array(8).fill(-.3),bottom:Array(8).fill(-.3)};
  for (const pixelRatio of [1,2]) {
    const rim=rasterizeFrameBevel({width:120,height:180,radius:18,step:1,profiles,pixelRatio});
    const pixel=(x,y)=>rim.data.subarray((Math.floor(y*pixelRatio)*rim.width+Math.floor(x*pixelRatio))*4,(Math.floor(y*pixelRatio)*rim.width+Math.floor(x*pixelRatio))*4+4);
    assert.equal(pixel(0,0)[3],0,'outside the rounded corner stays clear');
    assert.equal(pixel(60,90)[3],0,'the center stays clear');
    assert.ok(Math.abs(pixel(1.5,90)[3]-179)<=1,'left shadow is present');
    assert.ok(Math.abs(pixel(118.5,90)[3]-77)<=1,'right side retains its own strength');
    for (let angle=0;angle<=Math.PI/2;angle+=Math.PI/64) {
      const at=pixel(18-Math.cos(angle)*16,18-Math.sin(angle)*16);
      assert.ok(Math.abs(at[3]-179)<=1,'shadow continues through the full quarter-circle');
      assert.equal(at[0],0);
    }
  }
});

function embeddedArt({left=true,right=true,bottom=true,weakRight=false}={}) {
  const width=488,height=356,data=new Uint8ClampedArray(width*height*4);
  for(let y=0;y<height;y++) for(let x=0;x<width;x++) {
    let value=40;
    if(left&&x>=12&&x<19) value=155;
    if(right&&x>=width-19&&x<width-12) value=weakRight?53:155;
    if(bottom&&y>=height-8&&y<height-2) value=155;
    data.set([value,value,value,255],(y*width+x)*4);
  }
  return {data,width,height};
}

test('trims frame material outside the art while preserving the entire beveled enclosure', () => {
  const frame=detectEmbeddedArtFrame(embeddedArt());
  assert.ok(frame);
  assert.ok(frame.left.outer<12&&frame.left.outer>=10);
  assert.ok(frame.right.outer<12&&frame.right.outer>=10);
  assert.ok(frame.left.inner>=19);
  assert.ok(frame.bottom.outer<=2);
});

test('confirms a weak matching art edge using both transitions and the supported lower border', () => {
  const frame=detectEmbeddedArtFrame(embeddedArt({weakRight:true}));
  assert.ok(frame);
  assert.ok(Math.abs(frame.left.outer-frame.right.outer)<=1);
  assert.ok(frame.right.inner-frame.right.outer>=6);
});

test('never trims an illustration based on an isolated edge or unsupported bottom', () => {
  assert.equal(detectEmbeddedArtFrame(embeddedArt({left:false})),null);
  assert.equal(detectEmbeddedArtFrame(embeddedArt({right:false})),null);
  assert.equal(detectEmbeddedArtFrame(embeddedArt({bottom:false})),null);
  assert.equal(detectEmbeddedArtFrame(null),null);
});


test('fits visible rounded outer corners independently of a square interior', () => {
  for (const transparent of [true, false]) {
    const scan=printing({thickness:28}), radius=20;
    for (let y=0;y<scan.height;y++) for (let x=0;x<scan.width;x++) {
      const dx=Math.max(radius-x-.5,0,x+.5-(scan.width-radius));
      const dy=Math.max(radius-y-.5,0,y+.5-(scan.height-radius));
      if (dx*dx+dy*dy>radius*radius) scan.data.set(transparent?[0,0,0,0]:[255,255,255,255], (y*scan.width+x)*4);
    }
    const border=detectInnerFrameBorder(scan);
    assert.ok(Math.abs(border.outerRadius-radius)<=2, 'multiple corner samples recover the outer radius');
    assert.equal(border.innerRadius,0,'outer rounding does not round the interior');
    assert.equal(border.bounds.x,28);
  }
});
