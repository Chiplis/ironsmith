// Compare our filter with the saturation function extracted from native MSE.
import {readFile,mkdtemp,writeFile,rm} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {tmpdir} from 'node:os';
import {execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
import {MseImageRenderer} from '../src/lib/mse/image-renderer.mjs';
import {MseImage} from '../src/lib/mse/script-values.mjs';
import upstream from '../src/lib/mse/upstream.mjs';
const [checkout]=process.argv.slice(2);
if(!checkout)throw new Error('Usage: node mse-check-saturation.mjs NATIVE_MSE_CHECKOUT');
const revision=execFileSync('git',['-C',resolve(checkout),'rev-parse','HEAD'],{encoding:'utf8'}).trim();
assert.equal(revision,upstream.referenceImplementation.revision,'Expected pinned native MSE revision');
const source=await readFile(resolve(checkout,'src/gfx/image_effects.cpp'),'utf8');
const start=source.indexOf('void saturate('),end=source.indexOf('// ----------------------------------------------------------------------------- : Brightness',start);
assert.ok(start>=0&&end>start,'Native saturation function boundaries');
const directory=await mkdtemp(join(tmpdir(),'mse-saturation-'));
try{
  const cpp=`#include <vector>
#include <algorithm>
#include <cstdio>
#include <cassert>
using Byte=unsigned char;
int col(int x){return std::clamp(x,0,255);}
struct Image {std::vector<Byte> pixels; Byte* GetData(){return pixels.data();} int GetWidth(){return pixels.size()/3;} int GetHeight(){return 1;}};
${source.slice(start,end)}
int main(){for(double amount:{-2.0,-1.0,-0.999,-0.5,-0.003,0.0,0.003,0.5,0.999,1.0}){Image image;for(int r=0;r<256;r+=17)for(int g=0;g<256;g+=17)for(int b=0;b<256;b+=17){image.pixels.push_back(r);image.pixels.push_back(g);image.pixels.push_back(b);}saturate(image,amount);fwrite(image.pixels.data(),1,image.pixels.size(),stdout);}}
`;
  await writeFile(join(directory,'oracle.cpp'),cpp);
  execFileSync('c++',['-std=c++17',join(directory,'oracle.cpp'),'-o',join(directory,'oracle')]);
  const native=execFileSync(join(directory,'oracle'));
  const data=[];
  for(let r=0;r<256;r+=17)for(let g=0;g<256;g+=17)for(let b=0;b<256;b+=17)data.push(r,g,b,(r+g+b)%256);
  const renderer=new MseImageRenderer({loadAsset:async()=>({width:4096,height:1,data:new Uint8ClampedArray(data),hasAlpha:true})});
  let offset=0;
  for(const amount of [-2,-1,-.999,-.5,-.003,0,.003,.5,.999,1]){
    const result=await renderer.render(new MseImage('saturate',{image:new MseImage('asset',{path:{key:'fixture'}}),amount}));
    for(let i=0;i<data.length;i+=4){
      for(let c=0;c<3;c++)assert.equal(result.data[i+c],native[offset++],`amount=${amount}, pixel=${i/4}, channel=${c}`);
      assert.equal(result.data[i+3],data[i+3],'Alpha must remain unchanged');
    }
  }
  assert.equal(offset,native.length);
  console.log(JSON.stringify({revision,pixels:40960,rgbBytes:offset,alphaPreserved:true}));
}finally{await rm(directory,{recursive:true,force:true});}
