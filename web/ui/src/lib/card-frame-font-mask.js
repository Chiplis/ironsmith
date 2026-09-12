const alphabet="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,:;!?'-—–()/+−*•&©";
const banks=new Map();

function glyphBank(family,weight,italic=false,text='') {
  const words=String(text||'').replace(/\{[^}]+\}/g,' ').split(/\s+/).filter(Boolean);
  const extras=new Set();
  // Translations use letters outside the Latin alphabet below (á, ñ, ¿, …).
  for(const char of words.join(''))if(!alphabet.includes(char))extras.add(char);
  for(const word of words) {
    if(word.length<=6)extras.add(word);
    for(let i=0;i<word.length-1;i++) {extras.add(word.slice(i,i+2));if(i+2<word.length)extras.add(word.slice(i,i+3));}
  }
  const key=`${family}|${weight}|${italic}|${[...extras].join('|')}`;
  if(banks.has(key))return banks.get(key);
  const canvas=document.createElement('canvas');canvas.width=160;canvas.height=72;
  const ctx=canvas.getContext('2d',{willReadFrequently:true}),bank=[];
  ctx.font=`${italic?'italic ':''}${weight} 40px ${family}`;ctx.fillStyle='white';
  for(const char of [...alphabet,...extras]) {
    ctx.clearRect(0,0,160,72);ctx.fillText(char,12,52);
    const data=ctx.getImageData(0,0,160,72).data;
    let x0=160,y0=72,x1=0,y1=0;
    for(let y=0;y<72;y++)for(let x=0;x<160;x++)if(data[(y*160+x)*4+3]>72){x0=Math.min(x0,x);y0=Math.min(y0,y);x1=Math.max(x1,x);y1=Math.max(y1,y);}
    if(x0>x1)continue;
    const w=x1-x0+1,h=y1-y0+1,pixels=new Uint8Array(w*h);
    for(let y=0;y<h;y++)for(let x=0;x<w;x++)pixels[y*w+x]=data[((y+y0)*160+x+x0)*4+3]>72?1:0;
    bank.push({char,w,h,pixels});
  }
  banks.set(key,bank);if(banks.size>64)banks.delete(banks.keys().next().value);return bank;
}

export function glyphSimilarity(component, template) {
  if(Math.abs(Math.log((component.w/component.h)/(template.w/template.h)))>.55)return 0;
  let intersection=0,union=0;
  for(let y=0;y<component.h;y++)for(let x=0;x<component.w;x++) {
    const a=component.pixels[y*component.w+x];
    const b=template.pixels[Math.min(template.h-1,Math.floor((y+.5)*template.h/component.h))*template.w+Math.min(template.w-1,Math.floor((x+.5)*template.w/component.w))];
    if(a&&b)intersection++;if(a||b)union++;
  }
  return union?intersection/union:0;
}

// Frame material is not one colour. Split and gradient text boxes (dual-land
// promos, two-tone panels) shade from one side to the other, so a single paper
// level sits between the two halves: on the lighter side only glyph cores pass
// the ink threshold, leaving pale mana discs and anti-aliased halos behind, and
// the fill finds no donor paper to rebuild from. Estimate the level per tile
// and interpolate between tile centres. A uniform panel yields one level in
// every tile, so its masking is unchanged, and a region smaller than a tile
// keeps a single global level.
export function paperField({data,width,height},{tile=32}={}) {
  const cols=Math.max(1,Math.round(width/tile)),rows=Math.max(1,Math.round(height/tile));
  const levels=new Float64Array(cols*rows);
  for(let ty=0;ty<rows;ty++)for(let tx=0;tx<cols;tx++) {
    const x0=Math.floor(tx*width/cols),x1=Math.max(x0+1,Math.floor((tx+1)*width/cols));
    const y0=Math.floor(ty*height/rows),y1=Math.max(y0+1,Math.floor((ty+1)*height/rows));
    const bins=new Map();
    for(let y=y0;y<y1;y++)for(let x=x0;x<x1;x++) {
      const p=y*width+x,v=Math.round((data[p*4]+data[p*4+1]+data[p*4+2])/3/16);
      bins.set(v,(bins.get(v)||0)+1);
    }
    levels[ty*cols+tx]=([...bins].sort((a,b)=>b[1]-a[1])[0]?.[0]??0)*16;
  }
  return (x,y)=>{
    const fx=Math.min(cols-1,Math.max(0,x*cols/width-.5)),fy=Math.min(rows-1,Math.max(0,y*rows/height-.5));
    const x0=Math.floor(fx),y0=Math.floor(fy),x1=Math.min(cols-1,x0+1),y1=Math.min(rows-1,y0+1);
    const dx=fx-x0,dy=fy-y0;
    return levels[y0*cols+x0]*(1-dx)*(1-dy)+levels[y0*cols+x1]*dx*(1-dy)
      +levels[y1*cols+x0]*(1-dx)*dy+levels[y1*cols+x1]*dx*dy;
  };
}

// Fill only accepted glyph footprints. Boundary propagation retains local
// lighting; repeated relaxation avoids donor stripes and rectangular patches.
export function inpaintGlyphMask({data,width,height},mask) {
  const out=new Uint8ClampedArray(data),known=new Uint8Array(mask.length),pending=[];
  const paperAt=paperField({data,width,height});
  for(let p=0;p<mask.length;p++) {
    const light=(data[p*4]+data[p*4+1]+data[p*4+2])/3;
    // Nearby border strokes are preserved, but must not bleed into a glyph fill.
    // The comparison is against the paper beside this pixel, so a two-tone box
    // keeps donors on both sides of its divide.
    known[p]=!mask[p]&&Math.abs(light-paperAt(p%width,Math.floor(p/width)))<55?1:0;
  }
  for(let p=0;p<mask.length;p++)if(mask[p])pending.push(p);
  const neighbors=p=>{const x=p%width,y=Math.floor(p/width),ns=[];if(x)ns.push(p-1);if(x<width-1)ns.push(p+1);if(y)ns.push(p-width);if(y<height-1)ns.push(p+width);return ns;};
  for(let pass=0;pass<40;pass++) {
    const updates=[];
    for(const p of pending)if(!known[p]) {
      const ns=neighbors(p).filter(n=>known[n]);
      if(ns.length)updates.push([p,[0,1,2].map(c=>ns.reduce((s,n)=>s+out[n*4+c],0)/ns.length)]);
    }
    if(!updates.length)break;
    for(const [p,rgb] of updates){out.set([...rgb,255],p*4);known[p]=1;}
  }
  for(let pass=0;pass<12;pass++)for(const p of pending) {
    const ns=neighbors(p).filter(n=>known[n]);if(!ns.length)continue;for(let c=0;c<3;c++)out[p*4+c]=ns.reduce((s,n)=>s+out[n*4+c],0)/ns.length;
  }
  // Restore fine grain from nearby untouched paper after solving the fill.
  for(const p of pending) {
    const x=p%width,y=Math.floor(p/width);let donor=-1;
    for(let r=4;r<=14&&donor<0;r+=2)for(let i=0;i<8;i++) {
      const angle=((i+(p%8))*Math.PI/4),nx=x+Math.round(Math.cos(angle)*r),ny=y+Math.round(Math.sin(angle)*r),q=ny*width+nx;
      if(nx>0&&nx<width-1&&ny>0&&ny<height-1&&!mask[q]&&neighbors(q).every(n=>!mask[n])){donor=q;break;}
    }
    if(donor>=0)for(let c=0;c<3;c++) {
      const ns=neighbors(donor),mean=ns.reduce((s,n)=>s+data[n*4+c],0)/ns.length;
      out[p*4+c]+=Math.max(-6,Math.min(6,data[donor*4+c]-mean));
    }
  }
  return {data:out,width,height,mask};
}

// Panel bevels and rules that run along the edge of a text region are frame,
// not lettering. Left in the ink, a descender that touches one merges with it
// into a component far too wide to match any glyph, and that letter stays on
// the card under the replacement text (the "p" of Relámpago). Clear rows and
// columns in the outer band that are inked almost end to end.
export function clearEdgeRules(ink,width,height,{band=3,coverage=.7}={}) {
  for(let row=0;row<height;row++)if(row<band||row>=height-band) {
    let count=0;for(let px=0;px<width;px++)count+=ink[row*width+px];
    if(count>width*coverage)for(let px=0;px<width;px++)ink[row*width+px]=0;
  }
  for(let col=0;col<width;col++)if(col<band||col>=width-band) {
    let count=0;for(let py=0;py<height;py++)count+=ink[py*width+col];
    if(count>height*coverage)for(let py=0;py<height;py++)ink[py*width+col]=0;
  }
  return ink;
}

// Keep edge-rule evidence from the original scan through mask expansion.
// Otherwise dilation can erase the bevel beside an accepted letter even though
// that bevel was excluded from glyph matching.
export function expandGlyphMask(accepted,width,height,protectedPixels,radius=3) {
  const mask=new Uint8Array(accepted.length);
  for(let p=0;p<accepted.length;p++)if(accepted[p]) {
    const x=p%width,y=Math.floor(p/width);
    for(let dy=-radius;dy<=radius;dy++)for(let dx=-radius;dx<=radius;dx++) {
      const nx=x+dx,ny=y+dy;
      if(dx*dx+dy*dy<=radius*radius+1&&nx>=0&&nx<width&&ny>=0&&ny<height) {
        const q=ny*width+nx;
        if(!protectedPixels[q])mask[q]=1;
      }
    }
  }
  return mask;
}

// Midtone material (notably gold name bars) can carry white lettering too.
// Keep both contrast polarities there; shape matching still decides whether
// a bright component is a glyph rather than a highlight in the frame.
export function isPanelInk(r,g,b,paper,{outlined=false}={}) {
  const low=Math.min(r,g,b),high=Math.max(r,g,b),value=(r+g+b)/3;
  if(outlined)return low>165&&high-low<65;
  const paleInk=low>190&&high-low<65&&value>paper+45;
  return paleInk||(paper<115?value>paper+65:value<paper-55);
}

export function fontGuidedPanel(scan,{family,weight=400,italic=false,allowItalic=false,symbols=false,text='',section='',outlined=false}) {
  const {data,width,height}=scan;
  const paperAt=paperField(scan);
  const ink=new Uint8Array(width*height);
  for(let p=0;p<ink.length;p++) {
    const paper=paperAt(p%width,Math.floor(p/width));
    ink[p]=isPanelInk(data[p*4],data[p*4+1],data[p*4+2],paper,{outlined})?1:0;
  }
  const originalInk=ink.slice();
  clearEdgeRules(ink,width,height);
  const protectedPixels=originalInk.map((v,p)=>v&&!ink[p]?1:0);
  const bank=[...glyphBank(family,weight,italic,text),...(allowItalic?glyphBank(family,400,true,text):[])];
  const visited=new Uint8Array(ink.length),accepted=new Uint8Array(ink.length),components=[];
  for(let p=0;p<ink.length;p++)if(ink[p]&&!visited[p]) {
    const stack=[p],points=[];visited[p]=1;let x0=width,x1=0,y0=height,y1=0;
    while(stack.length) {
      const at=stack.pop(),x=at%width,y=Math.floor(at/width);points.push(at);
      x0=Math.min(x0,x);x1=Math.max(x1,x);y0=Math.min(y0,y);y1=Math.max(y1,y);
      for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++) {
        const nx=x+dx,ny=y+dy,n=ny*width+nx;
        if(nx>=0&&nx<width&&ny>=0&&ny<height&&!visited[n]&&ink[n]){visited[n]=1;stack.push(n);}
      }
    }
    const w=x1-x0+1,h=y1-y0+1;
    if(w>42||h>42||points.length<1)continue;
    const pixels=new Uint8Array(w*h);for(const at of points)pixels[(Math.floor(at/width)-y0)*w+at%width-x0]=1;
    let best=0,char='';
    for(const t of bank){const score=glyphSimilarity({w,h,pixels},t);if(score>best){best=score;char=t.char;}}
    components.push({points,x0,x1,y0,y1,w,h,best,char});
  }
  const matches=components.filter(c=>c.best>=.43&&c.h>=4);
  for(const c of components) {
    const punctuation=c.h<=3&&c.w>3&&c.best>.4;
    const tinyFooter=section==='footer'&&c.h<=7&&c.best>.25;
    const aligned=matches.some(m=>Math.abs(m.y1-c.y1)<4&&Math.min(Math.abs(m.x1-c.x0),Math.abs(c.x1-m.x0))<16);
    const dot=c.h<=3&&c.w<=3&&matches.some(m=>c.x0<=m.x1+4&&c.x1>=m.x0-4&&c.y0>=m.y0-6&&c.y1<=m.y1+4);
    // Registered single-line labels can use a different cut of the source
    // font. Include adjacent letter-shaped components even when their template
    // score is low; otherwise shorter translations expose the original suffix.
    const labelLetter=['title','type'].includes(section) && c.h>=4 && matches.some(m=>{
      const overlap=Math.min(c.y1,m.y1)-Math.max(c.y0,m.y0)+1;
      const gap=Math.max(0,c.x0-m.x1,m.x0-c.x1);
      return overlap>=Math.min(c.h,m.h)*.5 && c.h<=m.h*1.8
        && c.w<=c.h*2 && gap<=Math.max(8,m.h);
    });
    if(labelLetter||punctuation||tinyFooter||dot||c.best>=.38&&(c.h>=4||aligned)||symbols&&c.w>=6&&c.h>=6&&c.w/c.h>.65&&c.w/c.h<1.5)for(const p of c.points)accepted[p]=1;
  }
  if(symbols) {
    // Fit the complete circular disc, including its pale background, rather
    // than erasing only the numeral/skull inside it.
    const value=(x,y)=>{const at=(Math.round(y)*width+Math.round(x))*4;return (data[at]+data[at+1]+data[at+2])/3;};
    const circles=[];
    if(section==='title')for(const c of matches)if(c.x0>width*.82&&c.h>=6&&c.w<c.h*1.5) {
      const cx=(c.x0+c.x1)/2,cy=(c.y0+c.y1)/2,r=Math.min(height/2-1,c.h*.9);
      for(let y=Math.max(0,Math.floor(cy-r));y<=Math.min(height-1,Math.ceil(cy+r));y++)for(let x=Math.max(0,Math.floor(cx-r));x<=Math.min(width-1,Math.ceil(cx+r));x++)if(Math.hypot(x-cx,y-cy)<=r)accepted[y*width+x]=1;
    }
    for(let r=4;r<=Math.min(15,height/2-1);r++)for(let cy=r+1;cy<height-r-1;cy+=2)for(let cx=r+1;cx<width-r-1;cx+=2) {
      if(section==='title'&&cx<width*.7)continue;
      let support=0,contrast=0;
      const light=paperAt(cx,cy)<115;
      for(let a=0;a<16;a++) {
        const angle=a*Math.PI/8,dx=Math.cos(angle),dy=Math.sin(angle);
        const inner=value(cx+dx*(r-1),cy+dy*(r-1)),outer=value(cx+dx*(r+1),cy+dy*(r+1));
        const d=light?inner-outer:outer-inner;
        if(d>12)support++;contrast+=d;
      }
      if(support>=9&&contrast>160)circles.push({cx,cy,r,score:support*50+contrast});
    }
    circles.sort((a,b)=>b.score-a.score);
    const chosen=[];
    for(const c of circles)if(!chosen.some(o=>Math.hypot(o.cx-c.cx,o.cy-c.cy)<o.r+c.r)) {
      chosen.push(c);
      for(let y=Math.max(0,c.cy-c.r-1);y<=Math.min(height-1,c.cy+c.r+1);y++)for(let x=Math.max(0,c.cx-c.r-1);x<=Math.min(width-1,c.cx+c.r+1);x++)if(Math.hypot(x-c.cx,y-c.cy)<=c.r+1)accepted[y*width+x]=1;
    }
  }
  const mask=expandGlyphMask(accepted,width,height,protectedPixels,outlined?5:3);
  const result=inpaintGlyphMask(scan,mask);
  return {...result,matches:matches.length,method:'font-template'};
}
