import {readFile,writeFile,mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
import {execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
import {bleedRaster} from '../src/lib/mse/image-bleed.mjs';

const [reference,wxReference]=process.argv.slice(2);
if(!reference||!wxReference)throw new Error('Usage: mse-check-bleed.mjs MSE_SOURCE WX_IMAGE_CPP');
const sourcePath=resolve(reference,'src/gfx/generated_image.cpp');
const source=await readFile(sourcePath,'utf8'),wx=await readFile(resolve(wxReference),'utf8');
const functionStart=source.indexOf('Image BleedEdgedImage::generate(');
const start=source.indexOf('  int w = base_img.GetWidth()',functionStart),end=source.indexOf('// transfer metadata',start);
assert.ok(functionStart>=0&&start>functionStart&&end>start);
const compose=wx.indexOf('case wxIMAGE_ALPHA_BLEND_COMPOSE:'),loop=wx.indexOf('for (int i = 0; i < width; i++)',compose),body=wx.indexOf('{',loop);
assert.ok(compose>=0&&loop>compose&&body>loop);
let loopEnd=body+1,depth=1;
while(depth&&loopEnd<wx.length){if(wx[loopEnd]==='{')depth++;if(wx[loopEnd]==='}')depth--;loopEnd++;}
assert.equal(depth,0);
// The image adapter supplies storage and wx Paste. The edge sizing/mirroring
// and alpha composition below are copied from the reference source verbatim.
const cpp=`#include <iostream>
#include <vector>
#include <algorithm>
using namespace std;using Byte=unsigned char;using UInt=unsigned int;
#define _(s) s
const int MESSAGE_ERROR=0,wxIMAGE_ALPHA_BLEND_COMPOSE=0;
void queue_message(int,const char*){}
struct Color{int r,g,b,a;int Red(){return r;}int Green(){return g;}int Blue(){return b;}int Alpha(){return a;}};
struct Image{
 int w,h;vector<Byte> rgb,alpha;
 Image(int w,int h,bool=false):w(w),h(h),rgb(w*h*3){}
 int GetWidth()const{return w;}int GetHeight()const{return h;}
 void InitAlpha(){alpha.assign(w*h,255);}Byte* GetData(){return rgb.data();}Byte* GetAlpha(){return alpha.data();}
 void Paste(Image input,int x,int y,int){
  for(int row=0;row<input.h;row++){
   Byte* target_data=rgb.data()+((row+y)*w+x)*3;
   Byte* source_data=input.rgb.data()+row*input.w*3;
   Byte* alpha_target_data=alpha.data()+(row+y)*w+x;
   if(input.alpha.empty()){copy(source_data,source_data+input.w*3,target_data);fill(alpha_target_data,alpha_target_data+input.w,255);continue;}
   Byte* alpha_source_data=input.alpha.data()+row*input.w;int width=input.w;
   ${wx.slice(loop,loopEnd)}
  }
 }
};
using wxImage=Image;
Image bleed(Image base_img,double horizontal_size,double vertical_size,Color background_color){
${source.slice(start,end)}
return img;
}
int main(){int w,h,has;double x,y;Color bg;
while(cin>>w>>h>>has>>x>>y>>bg.r>>bg.g>>bg.b>>bg.a){
 Image input(w,h);if(has)input.InitAlpha();
 for(int i=0;i<w*h;i++){int r,g,b,a;cin>>r>>g>>b>>a;input.rgb[i*3]=r;input.rgb[i*3+1]=g;input.rgb[i*3+2]=b;if(has)input.alpha[i]=a;}
 Image output=bleed(input,x,y,bg);cout<<output.w<<' '<<output.h<<' ';
 for(int i=0;i<output.w*output.h;i++)cout<<int(output.rgb[i*3])<<' '<<int(output.rgb[i*3+1])<<' '<<int(output.rgb[i*3+2])<<' '<<(output.alpha.empty()?255:int(output.alpha[i]))<<' ';
 cout<<'\\n';
}}
`;
let seed=1729;
const random=n=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed%n;};
const cases=Array.from({length:96},(_,i)=>{
 const width=1+random(45),height=1+random(45),hasAlpha=i%2===0;
 const data=Uint8ClampedArray.from({length:width*height*4},(_,j)=>j%4!==3||hasAlpha?random(256):255);
 const sizes=[-1,0,0.1,0.25,0.9,2];
 return {width,height,hasAlpha,data,options:{horizontal:sizes[i%6],vertical:sizes[Math.floor(i/6)%6],background:[random(256),random(256),random(256),[0,128,255][i%3]]}};
});
const temp=await mkdtemp(join(tmpdir(),'mse-bleed-oracle-'));
try{
 await writeFile(join(temp,'oracle.cpp'),cpp);
 execFileSync('c++',['-std=c++17','-O2','-ffp-contract=off',join(temp,'oracle.cpp'),'-o',join(temp,'oracle')],{timeout:30000});
 const input=cases.map(c=>[c.width,c.height,Number(c.hasAlpha),c.options.horizontal,c.options.vertical,...c.options.background,...c.data].join(' ')).join('\n');
 const rows=execFileSync(join(temp,'oracle'),[],{input,encoding:'utf8',maxBuffer:20*1024*1024,timeout:30000}).trim().split('\n');
 assert.equal(rows.length,cases.length);let comparedBytes=0;
 cases.forEach((c,i)=>{const [width,height,...pixels]=rows[i].trim().split(/\s+/).map(Number),actual=bleedRaster(c,c.options);
  assert.deepEqual([actual.width,actual.height],[width,height],`Dimensions for case ${i}`);
  assert.deepEqual([...actual.data],pixels,`Pixels for case ${i}`);comparedBytes+=pixels.length;
 });
 console.log(JSON.stringify({status:'passed',reference:sourcePath,wxReference:resolve(wxReference),cases:cases.length,comparedBytes,scope:'Pixel sizing, background composition and mirrored edges; PNG metadata excluded'},null,2));
}finally{await rm(temp,{recursive:true,force:true});}
