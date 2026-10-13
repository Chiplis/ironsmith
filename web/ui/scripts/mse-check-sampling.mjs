import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { resizeRaster } from '../src/lib/mse/image-sampling.mjs';

const [reference] = process.argv.slice(2);
if (!reference) throw new Error('Usage: node scripts/mse-check-sampling.mjs NATIVE_MSE_SOURCE_DIRECTORY');
const sourcePath = resolve(reference, 'src/gfx/resample_image.cpp');
const source = await readFile(sourcePath, 'utf8');
const start = source.indexOf('void resample_pass('), body = source.indexOf('{', start);
if (start < 0 || body < 0) throw new Error('Native resample_pass function not found');
let end = body + 1, depth = 1;
while (depth && end < source.length) { if (source[end] === '{') depth++; if (source[end] === '}') depth--; end++; }
if (depth) throw new Error('Unterminated native resample_pass');
// Compile the actual upstream function against a small in-memory image adapter.
// No formula from the JavaScript implementation is substituted into this oracle.
const cpp = `#include <iostream>
#include <vector>
using Byte=unsigned char; using UInt=unsigned int;
const int shift=14;
struct Image {
  int w,h; mutable std::vector<Byte> rgb,alpha;
  Image(int w,int h):w(w),h(h),rgb(w*h*3){}
  bool HasAlpha()const{return !alpha.empty();}
  void InitAlpha(){alpha.assign(w*h,255);}
  Byte* GetData()const{return rgb.data();}
  Byte* GetAlpha()const{return alpha.data();}
};
${source.slice(start, end)}
int main(){
  int w,h,ow,oh,has;
  while(std::cin>>w>>h>>ow>>oh>>has){
    Image input(w,h); if(has)input.InitAlpha();
    for(int i=0;i<w*h;i++){int r,g,b,a;std::cin>>r>>g>>b>>a;input.rgb[i*3]=r;input.rgb[i*3+1]=g;input.rgb[i*3+2]=b;if(has)input.alpha[i]=a;}
    Image output(ow,oh);
    if(w==ow && h==oh) output=input;
    else if(h==oh)resample_pass(input,output,0,0,w,1,ow,1,h,w,ow);
    else {Image temp(ow,h);resample_pass(input,temp,0,0,w,1,ow,1,h,w,ow);resample_pass(temp,output,0,0,h,ow,oh,ow,ow,1,1);}
    for(int i=0;i<ow*oh;i++)std::cout<<int(output.rgb[i*3])<<' '<<int(output.rgb[i*3+1])<<' '<<int(output.rgb[i*3+2])<<' '<<(output.HasAlpha()?int(output.alpha[i]):255)<<' ';
    std::cout<<'\\n';
  }
}`;
let seed = 98131;
const random = max => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed % max; };
const cases = Array.from({ length: 128 }, (_, index) => {
  const width = 1 + random(13), height = 1 + random(11), outWidth = 1 + random(17), outHeight = 1 + random(15), hasAlpha = index % 2 === 0;
  const data = new Uint8ClampedArray(width * height * 4);
  for (let i = 0; i < data.length; i++) data[i] = i % 4 !== 3 || hasAlpha ? random(256) : 255;
  return { width, height, outWidth, outHeight, hasAlpha, data };
});
const temp = await mkdtemp(join(tmpdir(), 'ironsmith-mse-sampling-'));
try {
  await writeFile(join(temp, 'oracle.cpp'), cpp);
  execFileSync('c++', ['-std=c++17', '-O2', join(temp, 'oracle.cpp'), '-o', join(temp, 'oracle')], { timeout: 30000 });
  const input = cases.map(c => `${c.width} ${c.height} ${c.outWidth} ${c.outHeight} ${Number(c.hasAlpha)} ${[...c.data].join(' ')}`).join('\n');
  const output = execFileSync(join(temp, 'oracle'), [], { input, encoding: 'utf8', timeout: 30000 }).trim().split('\n');
  let bytes = 0;
  cases.forEach((c, index) => {
    const actual = resizeRaster(c, c.outWidth, c.outHeight);
    const expected = output[index].trim().split(/\s+/).map(Number);
    assert.deepEqual([...actual.data], expected, `Native mismatch for case ${index}: ${c.width}x${c.height} -> ${c.outWidth}x${c.outHeight}, alpha=${c.hasAlpha}`);
    bytes += expected.length;
  });
  console.log(JSON.stringify({ status: 'passed', reference: sourcePath, cases: cases.length, comparedBytes: bytes }, null, 2));
} finally { await rm(temp, { recursive: true, force: true }); }
