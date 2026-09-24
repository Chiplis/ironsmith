import test from 'node:test';
import assert from 'node:assert/strict';
import {locateSetSymbol} from '../src/lib/card-set-symbol.js';

test('registers a set silhouette independently of its rarity color',()=>{
  const width=488,height=680,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([220,220,210,255],p*4);
  const template={width:20,height:20,data:new Uint8ClampedArray(20*20*4)};
  for(let y=0;y<20;y++)for(let x=0;x<20;x++)if(Math.abs(x-9.5)+Math.abs(y-9.5)<9){template.data[(y*20+x)*4+3]=255;data.set([160,95,35,255],((390+y)*width+420+x)*4);}
  const found=locateSetSymbol({data,width,height},{x:30,y:382,width:426,height:36},template);
  assert.ok(found);
  assert.ok(Math.abs(found.x-420)<=2&&Math.abs(found.y-390)<=2);
  assert.equal(locateSetSymbol({data:new Uint8ClampedArray(width*height*4).fill(220),width,height},{x:30,y:382,width:426,height:36},template),null);
});

test('a wide boxed logo is registered as a complete enclosure',()=>{
  const width=488,height=680,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([225,225,220,255],p*4);
  const template={width:60,height:26,data:new Uint8ClampedArray(60*26*4)};
  for(let y=0;y<26;y++)for(let x=0;x<60;x++) {
    const mark=x<3||x>56||y<3||y>22||(x>10&&x<16)||(x>30&&x<35);
    if(mark){template.data[(y*60+x)*4+3]=255;data.set([25,25,25,255],((388+y)*width+391+x)*4);}
  }
  // A long translated type can cover the sampling baseline with dark ink.
  for(let y=398;y<=403;y++)for(let x=267;x<=332;x++)data.set([20,20,20,255],(y*width+x)*4);
  const found=locateSetSymbol({data,width,height},{x:26,y:381,width:436,height:38},template);
  assert.ok(found&&found.x<=391&&found.x+found.width>=451,JSON.stringify(found));
});

test('type lettering running into the symbol band is never taken for the symbol',()=>{
  // Finneas, Ace Archer: a long type line reaches the band the search covers,
  // and a letter shaped like the silhouette outscored the real symbol.
  const width=488,height=680,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([220,220,210,255],p*4);
  const template={width:20,height:20,data:new Uint8ClampedArray(20*20*4)};
  const diamond=(left,top,rgb)=>{for(let y=0;y<20;y++)for(let x=0;x<20;x++)if(Math.abs(x-9.5)+Math.abs(y-9.5)<9)data.set([...rgb,255],((top+y)*width+left+x)*4);};
  for(let y=0;y<20;y++)for(let x=0;x<20;x++)if(Math.abs(x-9.5)+Math.abs(y-9.5)<9)template.data[(y*20+x)*4+3]=255;
  // Lettering: strokes three pixels apart, then a perfect silhouette.
  for(let y=392;y<408;y++)for(const left of [328,336,344,352,360])for(let x=left;x<left+5;x++)data.set([20,20,20,255],(y*width+x)*4);
  diamond(370,390,[20,20,20]);
  // The real symbol, a weaker match, stands apart at x=420.
  diamond(420,390,[160,95,35]);
  for(let y=0;y<20;y+=4)for(let x=0;x<20;x++)data.set([220,220,210,255],((390+y)*width+420+x)*4);
  const found=locateSetSymbol({data,width,height},{x:30,y:382,width:426,height:36},template);
  assert.ok(found&&Math.abs(found.x-420)<=3,JSON.stringify(found));
});
