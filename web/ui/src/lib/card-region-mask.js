import {fontGuidedPanel,inpaintGlyphMask,hasOutlinedLightText} from './card-frame-font-mask.js';
import {profileSectionInk} from './card-printing-profile.js';
const scans=new Map(),patches=new Map();
function loadScan(url) {
  if(!scans.has(url)) {
    const promise=(async()=>{
      const image=new Image();image.crossOrigin='anonymous';image.referrerPolicy='no-referrer';image.src=url;await image.decode();
      const canvas=document.createElement('canvas');canvas.width=image.naturalWidth;canvas.height=image.naturalHeight;
      const ctx=canvas.getContext('2d',{willReadFrequently:true});ctx.drawImage(image,0,0);
      return ctx.getImageData(0,0,canvas.width,canvas.height);
    })();
    scans.set(url,promise);
    promise.catch(()=>scans.delete(url));
  }
  return scans.get(url);
}

// Recognize each line on the original scan, then fill the union once. Copying
// rectangular patches from separate original crops can restore a neighboring
// line's lettering where their padding overlaps.
export function maskRegisteredPixels(scan,regions,cleanPanel=fontGuidedPanel) {
  const {width:W,height:H,data}=scan,mask=new Uint8Array(W*H),samples=[];
  for(const {field,family,profile} of regions) {
    const points=new Set();
    let outlined=Boolean(field.outlined),fallback=false;
    for(const line of field.lines) {
      const x=Math.max(0,Math.floor(line.x*W)-2),y=Math.max(0,Math.floor(line.y*H)-2);
      const right=Math.min(W,Math.ceil((line.x+line.width)*W)+2),bottom=Math.min(H,Math.ceil((line.y+line.height)*H)+2);
      const w=right-x,h=bottom-y;
      if(w<=0||h<=0)throw new Error('Registered text lies outside the scan');
      const pixels=new Uint8ClampedArray(w*h*4);
      for(let row=0;row<h;row++)pixels.set(data.subarray(((y+row)*W+x)*4,((y+row)*W+x+w)*4),row*w*4);
      const region={data:pixels,width:w,height:h};
      const lineOutlined=field.outlined||hasOutlinedLightText(region);
      outlined ||= lineOutlined;
      const clean=cleanPanel(region,{family,weight:400,allowItalic:true,italic:field.kind==='flavor',symbols:field.kind==='rule',text:field.errata?line.text:field.printedText||field.text,section:field.kind==='name'?'title':field.kind,outlined:lineOutlined});
      // An unsafe template result cannot authorize replacement text. Rebuild
      // the complete registered line from surrounding paper instead, excluding
      // every other masked line from the donors in the shared fill below.
      const rebuild=!clean?.mask || clean.quality?.safe!==true;
      fallback ||= rebuild;
      for(let py=0;py<h;py++)for(let px=0;px<w;px++)if(rebuild||clean.mask[py*w+px]) {
        const p=(y+py)*W+x+px;mask[p]=1;points.add(p);
      }
    }
    samples.push({points,outlined,fallback,profile,kind:field.kind});
  }
  const result=inpaintGlyphMask(scan,mask);
  const inks=samples.map(({points,outlined,profile,kind})=>{
    const preferredInk=profileSectionInk(profile,kind);
    if(preferredInk==='light')return 'white';
    if(preferredInk==='dark')return 'rgb(0,0,0)';
    if(outlined)return 'white';
    const inkSamples=[...points].map(p=>{
      const at=p*4;
      return {contrast:Math.abs((data[at]+data[at+1]+data[at+2]-result.data[at]-result.data[at+1]-result.data[at+2])/3),rgb:[data[at],data[at+1],data[at+2]]};
    }).sort((a,b)=>b.contrast-a.contrast);
    const strongest=inkSamples.slice(0,Math.max(1,Math.floor(inkSamples.length*.15)));
    const channel=c=>strongest.map(s=>s.rgb[c]).sort((a,b)=>a-b)[Math.floor(strongest.length/2)];
    return strongest.length&&strongest[0].contrast>40?`rgb(${channel(0)},${channel(1)},${channel(2)})`:null;
  });
  return {...result,inks,fallbacks:samples.map(s=>s.fallback)};
}

// Publish one stationary cleaned scan and its ink colours atomically. The key
// includes all inputs used for recognition, including errata and source text.
export function maskRegisteredFrame(url,regions) {
  const key=JSON.stringify([url,regions]);
  if(patches.has(key))return patches.get(key);
  const promise=(async()=>{
    const scan=await loadScan(url),clean=maskRegisteredPixels(scan,regions);
    const canvas=document.createElement('canvas');canvas.width=scan.width;canvas.height=scan.height;
    canvas.getContext('2d').putImageData(new ImageData(clean.data,scan.width,scan.height),0,0);
    const image=canvas.toDataURL('image/png');
    const decoded=new Image();decoded.src=image;await decoded.decode();
    return {image,inks:clean.inks,fallbacks:clean.fallbacks};
  })();
  patches.set(key,promise);
  promise.catch(()=>patches.delete(key));
  if(patches.size>128)patches.delete(patches.keys().next().value);
  return promise;
}

// Retain the single-region API for callers that need a standalone patch.
export async function maskRegisteredRegion(url,field,family,profile) {
  const frame=await maskRegisteredFrame(url,[{field,family,profile}]);
  const scan=await loadScan(url),W=scan.width,H=scan.height,bounds=field.bounds;
  const x=Math.max(0,Math.floor(bounds.x*W)-3),y=Math.max(0,Math.floor(bounds.y*H)-3);
  const width=Math.min(W-x,Math.ceil(bounds.width*W)+6),height=Math.min(H-y,Math.ceil(bounds.height*H)+6);
  const image=new Image();image.src=frame.image;await image.decode();
  const canvas=document.createElement('canvas');canvas.width=width;canvas.height=height;
  canvas.getContext('2d').drawImage(image,x,y,width,height,0,0,width,height);
  return {image:canvas.toDataURL('image/png'),bounds:{x:x/W,y:y/H,width:width/W,height:height/H},ink:frame.inks[0]};
}
