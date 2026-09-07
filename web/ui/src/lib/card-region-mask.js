import {fontGuidedPanel,inpaintGlyphMask} from './card-frame-font-mask.js';
const scans=new Map(),patches=new Map();
function loadScan(url) {
  if(!scans.has(url))scans.set(url,(async()=>{
    const image=new Image();image.crossOrigin='anonymous';image.referrerPolicy='no-referrer';image.src=url;await image.decode();
    const canvas=document.createElement('canvas');canvas.width=image.naturalWidth;canvas.height=image.naturalHeight;
    const ctx=canvas.getContext('2d',{willReadFrequently:true});ctx.drawImage(image,0,0);
    return {canvas,ctx};
  })());
  return scans.get(url);
}
// Mask only registered printed text, leaving the rest of the scan untouched.
// Patches are shared between translations and repeated previews.
export function maskRegisteredRegion(url,field,family) {
  const key=JSON.stringify([url,field.lines,family]);
  if(patches.has(key))return patches.get(key);
  const promise=(async()=>{
    const {canvas,ctx}=await loadScan(url),W=canvas.width,H=canvas.height;
    const bounds=field.bounds;
    const x=Math.max(0,Math.floor(bounds.x*W)-3),y=Math.max(0,Math.floor(bounds.y*H)-3);
    const width=Math.min(W-x,Math.ceil(bounds.width*W)+6),height=Math.min(H-y,Math.ceil(bounds.height*H)+6);
    const patch=ctx.getImageData(x,y,width,height);
    const output=new Uint8ClampedArray(patch.data);
    const inkSamples=[];
    for(const line of field.lines) {
      const lx=Math.max(0,Math.floor(line.x*W)-x-2),ly=Math.max(0,Math.floor(line.y*H)-y-2);
      const w=Math.min(width-lx,Math.ceil(line.width*W)+4),h=Math.min(height-ly,Math.ceil(line.height*H)+4);
      if(w<=0||h<=0)continue;
      const data=new Uint8ClampedArray(w*h*4);
      for(let row=0;row<h;row++)data.set(patch.data.subarray(((ly+row)*width+lx)*4,((ly+row)*width+lx+w)*4),row*w*4);
      const region={data,width:w,height:h};
      let clean=fontGuidedPanel(region,{family,weight:400,allowItalic:true,italic:field.kind==='flavor',symbols:true,text:line.text,section:field.kind});
      if(!clean) {
        // OCR gives a tight ink rectangle. Contrast against the surrounding
        // paper supports lettering that our installed fonts cannot reproduce.
        const mask=new Uint8Array(w*h);
        for(let py=1;py<h-1;py++)for(let px=1;px<w-1;px++) {
          const at=(py*w+px)*4;
          let distance=0;
          for(let channel=0;channel<3;channel++) {
            const reference=(data[(py*w)*4+channel]+data[(py*w+w-1)*4+channel])/2;
            distance+=Math.abs(data[at+channel]-reference);
          }
          if(distance>105)mask[py*w+px]=1;
        }
        clean=inpaintGlyphMask(region,mask);
      }
      for(let row=0;row<h;row++)output.set(clean.data.subarray(row*w*4,(row+1)*w*4),((ly+row)*width+lx)*4);
      // The printed ink colour: glyph pixels that contrast most with the paper
      // the fill recovered, so light lettering on dark bars stays light.
      if(clean.mask)for(let p=0;p<w*h;p++)if(clean.mask[p]) {
        const at=p*4,paper=(clean.data[at]+clean.data[at+1]+clean.data[at+2])/3,light=(data[at]+data[at+1]+data[at+2])/3;
        inkSamples.push({contrast:Math.abs(light-paper),rgb:[data[at],data[at+1],data[at+2]]});
      }
    }
    const result=document.createElement('canvas');result.width=width;result.height=height;
    result.getContext('2d').putImageData(new ImageData(output,width,height),0,0);
    inkSamples.sort((a,b)=>b.contrast-a.contrast);
    // Anti-aliased edges dilute the colour; keep the solid glyph cores.
    const strongest=inkSamples.slice(0,Math.max(1,Math.floor(inkSamples.length*.15)));
    const channel=c=>strongest.map(s=>s.rgb[c]).sort((a,b)=>a-b)[Math.floor(strongest.length/2)];
    const ink=strongest.length&&strongest[0].contrast>40?`rgb(${channel(0)},${channel(1)},${channel(2)})`:null;
    return {image:result.toDataURL('image/png'),bounds:{x:x/W,y:y/H,width:width/W,height:height/H},ink};
  })();
  patches.set(key,promise);
  if(patches.size>512)patches.delete(patches.keys().next().value);
  return promise;
}
