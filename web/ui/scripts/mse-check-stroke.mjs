import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { strokeRaster } from '../src/lib/mse/image-stroke.mjs';
const [reference] = process.argv.slice(2);
if (!reference) throw new Error('Usage: node scripts/mse-check-stroke.mjs MSE_IMAGE_EFFECTS_CPP');
const source = await readFile(resolve(reference), 'utf8');
function extract(name) {
  const match = new RegExp(`(?:Byte|void|Image) ${name}\\(`).exec(source);
  assert.ok(match, name);
  let end=source.indexOf('{',match.index), depth=1, start=end++;
  while (depth && end<source.length) { if(source[end]==='{')depth++;if(source[end]==='}')depth--;end++; }
  assert.equal(depth,0);
  return source.slice(match.index,end);
}
const fixture = (w,h,alpha) => ({width:w,height:h,hasAlpha:alpha,data:Uint8ClampedArray.from({length:w*h*4},(_,i)=>i%4===3 ? (Math.floor(i/4)*73+19)%256 : 100)});
const cases=[];
for (const [w,h] of [[1,1],[2,7],[9,5]]) for (const alpha of [false,true]) for (const radius of [0,1,3]) for (const blur of [0,1,4]) for (const opacity of [0,127,255]) cases.push({w,h,alpha,radius,blur,opacity});
const make = extract('make_stroke_image').replace(/  \/\/ transfer metadata[\s\S]*?(?=  return s_img;)/,'');
const cpp=`#include <algorithm>
#include <vector>
#include <iostream>
using namespace std; using Byte=unsigned char;
struct Color { Byte r,g,b,a; Byte Red(){return r;} Byte Green(){return g;} Byte Blue(){return b;} Byte Alpha(){return a;} };
struct Image { int w,h; vector<Byte> rgb,alpha; Image(int w,int h,bool clear=false):w(w),h(h),rgb(w*h*3){} bool HasAlpha(){return !alpha.empty();} void InitAlpha(){alpha.resize(w*h,255);} int GetWidth(){return w;}int GetHeight(){return h;}Byte* GetData(){return rgb.data();}Byte* GetAlpha(){return alpha.data();} };
using wxImage=Image;
void set_alpha(Image& img,double){img.InitAlpha();}
${extract('blur_pixel_alpha')}
${extract('blur_image_alpha')}
${extract('thicken_pixel_alpha_3x3')}
${extract('thicken_pixel_alpha_5x5')}
${extract('thicken_pixel_alpha_7x7')}
${extract('thicken_image_alpha')}
${make}
int main(){int cases[][6]={${cases.map(c=>`{${c.w},${c.h},${+c.alpha},${c.radius},${c.blur},${c.opacity}}`).join(',')}};
for(auto& c:cases){Image img(c[0],c[1]);if(c[2]){img.InitAlpha();for(int i=0;i<c[0]*c[1];i++)img.alpha[i]=(i*73+19)%256;}
Image out=make_stroke_image(img,Color{17,89,203,(Byte)c[5]},c[3],c[4]);
for(int i=0;i<out.w*out.h;i++){for(int j=0;j<3;j++)cout.put(out.rgb[i*3+j]);cout.put(out.alpha[i]);}}
}`;
const temp=await mkdtemp(join(tmpdir(),'ironsmith-mse-stroke-'));
try {
  await writeFile(join(temp,'oracle.cpp'),cpp);
  execFileSync('c++',['-std=c++17','-O2',join(temp,'oracle.cpp'),'-o',join(temp,'oracle')],{timeout:30000});
  const native=execFileSync(join(temp,'oracle'),[],{maxBuffer:4000000});
  let offset=0;
  for(const c of cases){
    const actual=strokeRaster(fixture(c.w,c.h,c.alpha),[17,89,203,c.opacity],c.radius,c.blur);
    assert.deepEqual(Buffer.from(actual.data),native.subarray(offset,offset+actual.data.length),JSON.stringify(c));
    offset+=actual.data.length;
  }
  assert.equal(offset,native.length);
  console.log(JSON.stringify({status:'passed',cases:cases.length,bytes:offset,reference:resolve(reference)},null,2));
} finally {await rm(temp,{recursive:true,force:true});}
