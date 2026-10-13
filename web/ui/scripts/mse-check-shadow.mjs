import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { dropShadowRaster } from '../src/lib/mse/image-shadow.mjs';
const [reference]=process.argv.slice(2);
if(!reference)throw new Error('Usage: node scripts/mse-check-shadow.mjs MSE_GENERATED_IMAGE_CPP');
const source=await readFile(resolve(reference),'utf8');
const extract=signature=>{
  const start=source.indexOf(signature);assert.ok(start>=0);
  let end=source.indexOf('{',start)+1,depth=1;
  while(depth&&end<source.length){if(source[end]==='{')depth++;if(source[end]==='}')depth--;end++;}
  assert.equal(depth,0);return source.slice(start,end);
};
const gaussian=extract('UInt gaussian_blur(');
const generate=extract('Image DropShadowImage::generate(');
const body=generate.slice(generate.indexOf('  int w ='),generate.lastIndexOf('  return img;'));
const fixtures=[];
for(const [w,h] of [[1,1],[7,11],[32,19]])for(const blur of [0.03,0.12,0.5])for(const alpha of [0,0.5,1,1.5])for(const [x,y] of [[0,0],[-0.2,0.1],[0.15,-0.3]])fixtures.push({w,h,blur,alpha,x,y});
const cpp=`#include <algorithm>
#include <vector>
#include <memory>
#include <cstring>
#include <cmath>
#include <iostream>
using namespace std;using Byte=unsigned char;using UInt=unsigned int;
struct Color{int Red(){return 17;}int Green(){return 89;}int Blue(){return 203;}};
struct Image{int w,h;vector<Byte> rgb,alpha;Image(int w,int h):w(w),h(h),rgb(w*h*3),alpha(w*h){for(int i=0;i<w*h;i++){alpha[i]=(i*73+19)%256;for(int c=0;c<3;c++)rgb[i*3+c]=(i*29+c*37)%256;}}int GetWidth(){return w;}int GetHeight(){return h;}Byte* GetAlpha(){return alpha.data();}Byte* GetData(){return rgb.data();}};
${gaussian}
int main(){double fixtures[][6]={${fixtures.map(f=>`{${f.w},${f.h},${f.blur},${f.alpha},${f.x},${f.y}}`).join(',')}};
for(auto& f:fixtures){Image img(f[0],f[1]);double shadow_blur_radius=f[2],shadow_alpha=f[3],offset_x=f[4],offset_y=f[5];Color shadow_color;
${body}
for(int i=0;i<img.w*img.h;i++){for(int c=0;c<3;c++)cout.put(img.rgb[i*3+c]);cout.put(img.alpha[i]);}}}`;
const temp=await mkdtemp(join(tmpdir(),'ironsmith-mse-shadow-'));
try{
  await writeFile(join(temp,'oracle.cpp'),cpp);
  execFileSync('c++',['-std=c++17','-O2',join(temp,'oracle.cpp'),'-o',join(temp,'oracle')],{timeout:30000});
  const native=execFileSync(join(temp,'oracle'),[],{maxBuffer:4000000});let offset=0;
  for(const f of fixtures){
    const input={width:f.w,height:f.h,hasAlpha:true,data:Uint8ClampedArray.from({length:f.w*f.h*4},(_,i)=>i%4===3?(Math.floor(i/4)*73+19)%256:(Math.floor(i/4)*29+i%4*37)%256)};
    const actual=dropShadowRaster(input,{offsetX:f.x,offsetY:f.y,alpha:f.alpha,blur:f.blur,color:[17,89,203,255]});
    assert.deepEqual(Buffer.from(actual.data),native.subarray(offset,offset+actual.data.length),JSON.stringify(f));offset+=actual.data.length;
  }
  assert.equal(offset,native.length);console.log(JSON.stringify({status:'passed',cases:fixtures.length,bytes:offset,reference:resolve(reference)},null,2));
}finally{await rm(temp,{recursive:true,force:true});}
