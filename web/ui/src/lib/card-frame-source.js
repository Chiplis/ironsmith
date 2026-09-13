import {inpaintGlyphMask} from './card-frame-font-mask.js';
// Keep the original printing everywhere except its editable text regions.
// The caller supplies the existing glyph-removal/inpainting implementation.
function* maskSourceFrameSteps(scan, boxes, stats, statsPanel, panels = {}) {
  const {width,height,data}=scan, output=data.slice(), mask=new Uint8Array(width*height);
  const regions=[];
  for(const name of ['title','type','rules']) {
    if(name==='rules'&&panels.preserveRules)continue;
    const b=boxes[name];
    if(!b)return null;
    const text=panels.textBounds?.[name];
    if(text) {
      // Recognition may miss a suffix. Keep its measured vertical band, but
      // inspect the complete label up to the independently registered symbol.
      const stop=name==='title'?panels.manaMatch?.symbols[0]?.x:panels.setSymbol?.x;
      // Without a registered symbol, extending across unknown pixels can
      // capture a boxed set logo as lettering. Use the complete measured line
      // in that case; only an independent symbol anchor permits expansion.
      const right=Math.floor(Math.min(b.x+b.width,stop!=null?stop-1:text.x+text.width+3));
      regions.push({name,x:text.x-3,y:text.y-3,width:Math.max(1,right-(text.x-3)),height:text.height+6});
      continue;
    }
    const integrated=panels[name]==='integrated';
    const inset=integrated?0:name==='rules'?6:7;
    const vertical=integrated?(name==='title'?-2:0):5;
    regions.push({name,x:Math.ceil(b.x+inset),y:Math.ceil(b.y+vertical),
      width:Math.floor((name==='type'?Math.min(b.width,(panels.setSymbol?.x??width*.855)-4-b.x):name==='title'&&integrated?Math.max(b.width,width*.93-b.x):b.width)-inset*2),height:Math.floor(b.height-vertical*2)});
  }
  if(stats)regions.push({name:'stats',x:stats.x-3,y:stats.y-3,width:stats.width+6,height:stats.height+6});
  const inStats=(x,y)=>statsPanel&&x>=statsPanel.x&&x<statsPanel.x+statsPanel.width&&y>=statsPanel.y&&y<statsPanel.y+statsPanel.height;
  for(const r of regions) {
    r.x=Math.max(0,r.x);r.y=Math.max(0,r.y);
    r.width=Math.min(width-r.x,r.width);r.height=Math.min(height-r.y,r.height);
    if(r.width<1||r.height<1)return null;
    const patch=new Uint8ClampedArray(r.width*r.height*4);
    for(let y=0;y<r.height;y++)patch.set(data.subarray(((r.y+y)*width+r.x)*4,((r.y+y)*width+r.x+r.width)*4),y*r.width*4);
    const excludedPixels=new Uint8Array(r.width*r.height);
    if(r.name==='rules'&&statsPanel)for(let y=0;y<r.height;y++)for(let x=0;x<r.width;x++)if(inStats(r.x+x,r.y+y))excludedPixels[y*r.width+x]=1;
    let clean=yield {kind:'panel',scan:{data:patch,width:r.width,height:r.height},options:{removeSeparators:r.name==='rules',minimumCleanFraction:.02,section:r.name,excludedPixels,protectBottomBoundary:r.name==='rules'}};
    if (!clean) return null;
    if (clean.quality?.safe === false) {
      panels.onUnsafeMask?.({section:r.name,quality:clean.quality});
      return null;
    }
    if(r.name==='title'&&!panels.fontGuided) {
      const donors=[];
      for(let p=0;p<r.width*r.height;p++)if(!clean.mask[p]&&p%r.width<r.width*.65)donors.push(p);
      if(donors.length)for(let y=0;y<r.height;y++)for(let x=0;x<r.width;x++)if(r.x+x>width*.78) {
        const p=y*r.width+x,d=donors[(Math.imul(p+1,2654435761)>>>0)%donors.length];
        clean.mask[p]=1;clean.data.set(patch.subarray(d*4,d*4+4),p*4);
      }
    }
    for(let y=0;y<r.height;y++)for(let x=0;x<r.width;x++) {
      if(r.name==='rules'&&inStats(r.x+x,r.y+y))continue;
      if(r.name==='title'&&panels.manaMatch?.symbols.some(b=>r.x+x>=b.x-3&&r.x+x<b.x+b.width+3&&r.y+y>=b.y-3&&r.y+y<b.y+b.height+3))continue;
      const local=y*r.width+x,at=(r.y+y)*width+r.x+x;
      if(clean.mask[local]) {
        mask[at]=1;output.set(clean.data.subarray(local*4,local*4+4),at*4);
      }
    }
  }
  if(panels.manaMatch) {
    const symbols=panels.manaMatch.symbols;
    const x0=Math.max(0,Math.floor(Math.min(...symbols.map(b=>b.x))-4)),y0=Math.max(0,Math.floor(Math.min(...symbols.map(b=>b.y))-4));
    const w=Math.min(width-x0,Math.ceil(Math.max(...symbols.map(b=>b.x+b.width))+4-x0)),h=Math.min(height-y0,Math.ceil(Math.max(...symbols.map(b=>b.y+b.height))+4-y0));
    const pixels=new Uint8ClampedArray(w*h*4),symbolMask=new Uint8Array(w*h);
    for(let y=0;y<h;y++)pixels.set(output.subarray(((y+y0)*width+x0)*4,((y+y0)*width+x0+w)*4),y*w*4);
    symbols.forEach((b,i)=>{
      const svg=panels.icons[i];
      for(let y=Math.floor(b.y-2);y<b.y+b.height+2;y++)for(let x=Math.floor(b.x-2);x<b.x+b.width+2;x++) {
        if(x<x0||x>=x0+w||y<y0||y>=y0+h)continue;
        const u=Math.min(svg.width-1,Math.max(0,Math.floor((x-b.x+2)/(b.width+4)*svg.width)));
        const v=Math.min(svg.height-1,Math.max(0,Math.floor((y-b.y+2)/(b.height+4)*svg.height)));
        if(svg.data[(v*svg.width+u)*4+3]>32)symbolMask[(y-y0)*w+x-x0]=1;
      }
    });
    const filled=yield {kind:'inpaint',scan:{data:pixels,width:w,height:h},mask:symbolMask};
    for(let y=0;y<h;y++)for(let x=0;x<w;x++)if(symbolMask[y*w+x]) {
      const p=(y+y0)*width+x+x0;output.set(filled.data.subarray((y*w+x)*4,(y*w+x)*4+4),p*4);mask[p]=1;
    }
  }
  // Scryfall's rounded JPEG corners contain white outside the physical card.
  // Clear only that corner-connected background, never white frame material.
  const seen=new Uint8Array(width*height),queue=[0,width-1,(height-1)*width,width*height-1];
  while(queue.length) {
    const p=queue.pop();if(seen[p])continue;seen[p]=1;
    const x=p%width,y=Math.floor(p/width);
    if(Math.min(x,width-1-x)>width*.08||Math.min(y,height-1-y)>height*.06)continue;
    if([0,1,2].some(c=>data[p*4+c]<242))continue;
    output[p*4+3]=0;mask[p]=1;
    if(x>0)queue.push(p-1);if(x<width-1)queue.push(p+1);
    if(y>0)queue.push(p-width);if(y<height-1)queue.push(p+width);
  }
  return {data:output,width,height,mask};
}

export function maskSourceFrame(scan, boxes, stats, statsPanel, cleanPanel, panels = {}) {
  const steps = maskSourceFrameSteps(scan, boxes, stats, statsPanel, panels);
  let step = steps.next();
  while (!step.done) {
    const job = step.value;
    step = steps.next(job.kind === 'panel' ? cleanPanel(job.scan, job.options) : inpaintGlyphMask(job.scan, job.mask));
  }
  return step.value;
}

export async function maskSourceFrameAsync(scan, boxes, stats, statsPanel, cleanPanel, panels = {}, inpaint = inpaintGlyphMask) {
  const steps = maskSourceFrameSteps(scan, boxes, stats, statsPanel, panels);
  let step = steps.next();
  while (!step.done) {
    const job = step.value;
    step = steps.next(await (job.kind === 'panel' ? cleanPanel(job.scan, job.options) : inpaint(job.scan, job.mask)));
  }
  return step.value;
}
