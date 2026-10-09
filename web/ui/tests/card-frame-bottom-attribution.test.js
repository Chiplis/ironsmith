import test from 'node:test';
import assert from 'node:assert/strict';
import {detectPanelBounds} from '../src/lib/card-frame-colors.js';

test('a dark attribution cannot replace a lighter full-width textbox bottom',()=>{
  const width=488,height=680,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([185,185,185,255],p*4);
  for(let y=425;y<632;y++)for(let x=32;x<455;x++)data.set([220,220,220,255],(y*width+x)*4);
  // Ink spans enough of the row to beat the ordinary 65% edge threshold,
  // but leaves the right quarter blank, like a flavor-text attribution.
  for(let y=609;y<616;y++)for(let x=70;x<320;x++)data.set([0,0,0,255],(y*width+x)*4);
  const box=detectPanelBounds({width,height,data},'rules',{typePanel:'panel'});
  assert.ok(box);
  assert.ok(box.y+box.height>=630,JSON.stringify(box));
});
