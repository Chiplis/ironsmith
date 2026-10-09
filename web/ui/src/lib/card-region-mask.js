import {fontGuidedPanel,inpaintGlyphMask,hasOutlinedLightText,statsGlyphMask,paperField} from './card-frame-font-mask.js';
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

const orientedScans=new Map();
// Normalize the scan and its registered coordinates together. CSS restores the
// printed orientation only after text layout and cleanup in this coordinate system.
export function orientedRegisteredScan(url,rotation) {
  if(!rotation)return Promise.resolve(url);
  const key=url+'|'+rotation;
  if(orientedScans.has(key))return orientedScans.get(key);
  const promise=loadScan(url).then(scan=>{
    const source=document.createElement('canvas');source.width=scan.width;source.height=scan.height;
    source.getContext('2d').putImageData(scan,0,0);
    const canvas=document.createElement('canvas'),quarter=Math.abs(rotation)%180===90;
    canvas.width=quarter?scan.height:scan.width;canvas.height=quarter?scan.width:scan.height;
    const ctx=canvas.getContext('2d');ctx.translate(canvas.width/2,canvas.height/2);ctx.rotate(rotation*Math.PI/180);ctx.drawImage(source,-scan.width/2,-scan.height/2);
    return canvas.toDataURL('image/png');
  });
  orientedScans.set(key,promise);promise.catch(()=>orientedScans.delete(key));
  if(orientedScans.size>128)orientedScans.delete(orientedScans.keys().next().value);
  return promise;
}

// Recognize each line on the original scan, then fill the union once. Copying
// rectangular patches from separate original crops can restore a neighboring
// line's lettering where their padding overlaps.
export function maskRegisteredPixels(scan,regions,cleanPanel=fontGuidedPanel) {
  const {width:W,height:H,data}=scan,mask=new Uint8Array(W*H),samples=[],badges=[];
  for(const {field,family,profile} of regions) {
    const points=new Set();
    let outlined=Boolean(field.outlined),fallback=false;
    for(const line of field.lines) {
      const x=Math.max(0,Math.floor(line.x*W)-2),y=Math.max(0,Math.floor(line.y*H)-2);
      const right=Math.min(W,Math.ceil((line.x+line.width)*W)+2),bottom=Math.min(H,Math.ceil((line.y+line.height)*H)+2);
      const w=right-x,h=bottom-y;
      if(w<=0||h<=0)throw new Error('Registered text lies outside the scan');
      const pixels=new Uint8ClampedArray(w*h*4);
      const badgeMask=['stats','tier-stats','loyalty-cost'].includes(field.kind)?new Uint8Array(w*h):null;
      for(let row=0;row<h;row++)pixels.set(data.subarray(((y+row)*W+x)*4,((y+row)*W+x+w)*4),row*w*4);
      let region={data:pixels,width:w,height:h};
      if(field.rotation===180) {
        const upright=new Uint8ClampedArray(pixels.length);
        for(let p=0;p<w*h;p++)upright.set(pixels.subarray(p*4,p*4+4),(w*h-1-p)*4);
        region={data:upright,width:w,height:h};
      } else if(field.rotation===90) {
        const upright=new Uint8ClampedArray(pixels.length);
        for(let py=0;py<h;py++)for(let px=0;px<w;px++)upright.set(pixels.subarray((py*w+px)*4,(py*w+px)*4+4),((w-1-px)*h+py)*4);
        region={data:upright,width:h,height:w};
      }
      const polarity=field.polarity || profileSectionInk(profile,field.kind);
      const lineOutlined=field.polarity==='dark'?false:field.outlined??hasOutlinedLightText(region);
      outlined ||= lineOutlined;
      const clean=badgeMask?{mask:statsGlyphMask(region),quality:{safe:true}}:field.opaqueHeader||field.opaqueLettering||field.rebuildPrintedLines?null:cleanPanel(region,{family,weight:400,allowItalic:true,italic:field.kind==='flavor',symbols:field.kind==='rule',text:field.errata?line.text:field.printedText||field.text,section:field.kind==='name'?'title':field.kind,outlined:lineOutlined,polarity});
      // An unsafe template result cannot authorize replacement text. Rebuild
      // the complete registered line from surrounding paper instead, excluding
      // every other masked line from the donors in the shared fill below.
      const rebuild=!clean?.mask || clean.quality?.safe!==true;
      const paper=rebuild&&polarity==='dark'?paperField({data:pixels,width:w,height:h}):null;
      fallback ||= rebuild;
      for(let py=0;py<h;py++)for(let px=0;px<w;px++)if(rebuild||clean.mask[field.rotation===180?w*h-1-(py*w+px):field.rotation===90?(w-1-px)*h+py:py*w+px]) {
        // Failed template matching still identifies the printed line, not a
        // rectangle of disposable paper. Keep untouched paper inside that
        // line as donors so adjacent artwork cannot bleed into the panel.
        if(paper) {
          let ink=false;
          for(let dy=-1;dy<=1&&!ink;dy++)for(let dx=-1;dx<=1;dx++) {
            const nx=px+dx,ny=py+dy;
            if(nx<0||nx>=w||ny<0||ny>=h)continue;
            const at=(ny*w+nx)*4;
            const light=(pixels[at]+pixels[at+1]+pixels[at+2])/3;
            if(light<paper(nx,ny)-22){ink=true;break;}
          }
          if(!ink)continue;
        }
        // A shared reminder strip has pale glyphs on a dark frame rail.
        // Retain its paper as donors: erasing the entire strip would pull the
        // adjacent artwork into it. Include antialiasing beside each glyph.
        if(field.sharedRule&&field.polarity==='light') {
          let pale=false;
          for(let dy=-1;dy<=1&&!pale;dy++)for(let dx=-1;dx<=1;dx++) {
            const nx=px+dx,ny=py+dy;
            if(nx<0||nx>=w||ny<0||ny>=h)continue;
            const at=(ny*w+nx)*4;
            if(Math.min(pixels[at],pixels[at+1],pixels[at+2])>140){pale=true;break;}
          }
          if(!pale)continue;
        }
        if(field.sharedRule&&field.polarity==='dark') {
          let dark=false;
          for(let dy=-1;dy<=1&&!dark;dy++)for(let dx=-1;dx<=1;dx++) {
            const nx=px+dx,ny=py+dy;
            if(nx<0||nx>=w||ny<0||ny>=h)continue;
            const at=(ny*w+nx)*4;
            if(Math.max(pixels[at],pixels[at+1],pixels[at+2])<110){dark=true;break;}
          }
          if(!dark)continue;
        }
        if(field.protectedBounds?.some(b=>(x+px)/W>=b.x&&(x+px)/W<=b.x+b.width&&(y+py)/H>=b.y&&(y+py)/H<=b.y+b.height))continue;
        const p=(y+py)*W+x+px;mask[p]=1;points.add(p);
        if(badgeMask)badgeMask[py*w+px]=1;
      }
      if(badgeMask)badges.push({x,y,w,h,scan:{data:pixels,width:w,height:h},mask:badgeMask});
    }
    samples.push({points,outlined,fallback,profile,polarity:field.polarity,kind:field.kind});
  }
  const result=inpaintGlyphMask(scan,mask);
  // A small black shield sits inside pale paper. Its own crop supplies the
  // paper estimate; a card-wide estimate would reject every dark donor.
  for(const badge of badges) {
    const fill=inpaintGlyphMask(badge.scan,badge.mask);
    for(let p=0;p<badge.mask.length;p++)if(badge.mask[p]) {
      const destination=((badge.y+Math.floor(p/badge.w))*W+badge.x+p%badge.w)*4;
      result.data.set(fill.data.subarray(p*4,p*4+4),destination);
    }
  }
  const inks=samples.map(({points,outlined,profile,kind,polarity})=>{
    const preferredInk=polarity||profileSectionInk(profile,kind);
    if(outlined&&polarity!=='dark')return 'white';
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
  return {...result,inks,outlines:samples.map(s=>s.outlined),fallbacks:samples.map(s=>s.fallback)};
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
    return {image,inks:clean.inks,outlines:clean.outlines,fallbacks:clean.fallbacks};
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
