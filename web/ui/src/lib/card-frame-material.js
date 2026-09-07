// Reconstruct one source-resolution frame atlas. Geometry excludes printed
// content; regional medians carry lighting independently of fine grain.
export function reconstructFrameMaterial(scan, bounds, boxes, stats) {
  const {data,width,height}=scan;
  if (!bounds || !boxes) return null;
  const mask=new Uint8Array(width*height), pad=3;
  const excluded=Object.entries(boxes).map(([name,b])=>name==='title'||name==='type'
    ? {...b,x:bounds.x,width:bounds.width}
    : name==='art' ? {...b,y:boxes.title.y+boxes.title.height,height:boxes.type.y-boxes.title.y-boxes.title.height} : b);
  // Artist/collector lines and P/T are not frame material.
  excluded.push({x:0,y:boxes.rules.y+boxes.rules.height,width:width,height:height});
  if(stats) excluded.push(stats);
  for(let y=0;y<height;y++) for(let x=0;x<width;x++) {
    mask[y*width+x]=x<bounds.x+5||x>=bounds.x+bounds.width-5||y<bounds.y+5||y>=bounds.y+bounds.height-5
      || data[(y*width+x)*4+3]<240 || excluded.some(b=>x>=b.x-pad&&x<b.x+b.width+pad&&y>=b.y-pad&&y<b.y+b.height+pad) ? 1:0;
  }
  const step=24,cols=Math.ceil(width/step),rows=Math.ceil(height/step);
  const cells=Array.from({length:cols*rows},()=>[[],[],[]]);
  let clean=0;
  for(let y=0;y<height;y++) for(let x=0;x<width;x++) if(!mask[y*width+x]) {
    clean++;
    const cell=cells[Math.floor(y/step)*cols+Math.floor(x/step)];
    for(let c=0;c<3;c++) cell[c].push(data[(y*width+x)*4+c]);
  }
  if(clean<width*height*.01) return null;
  const med=a=>a.sort((x,y)=>x-y)[Math.floor(a.length/2)];
  const field=cells.map(c=>c[0].length>=12?c.map(med):null);
  const supported=field.flatMap((rgb,i)=>rgb?[{rgb,x:i%cols,y:Math.floor(i/cols)}]:[]);
  if(supported.length<4) return null;
  for(let y=0;y<rows;y++) for(let x=0;x<cols;x++) if(!field[y*cols+x]) {
    const near=supported.map(p=>({...p,d:(p.x-x)**2+(p.y-y)**2})).sort((a,b)=>a.d-b.d).slice(0,6);
    const weight=near.reduce((s,p)=>s+1/(p.d+.25),0);
    field[y*cols+x]=[0,1,2].map(c=>near.reduce((s,p)=>s+p.rgb[c]/(p.d+.25),0)/weight);
  }
  const shade=(x,y,c)=>{
    const fx=Math.max(0,Math.min(cols-1,x/step-.5)),fy=Math.max(0,Math.min(rows-1,y/step-.5));
    const ix=Math.floor(fx),iy=Math.floor(fy),tx=fx-ix,ty=fy-iy;
    return field[iy*cols+ix][c]*(1-tx)*(1-ty)+field[iy*cols+Math.min(cols-1,ix+1)][c]*tx*(1-ty)
      +field[Math.min(rows-1,iy+1)*cols+ix][c]*(1-tx)*ty+field[Math.min(rows-1,iy+1)*cols+Math.min(cols-1,ix+1)][c]*tx*ty;
  };
  const tile=6,donors=[];
  for(let y=bounds.y+5;y<bounds.y+bounds.height-tile-5;y+=3) for(let x=bounds.x+5;x<bounds.x+bounds.width-tile-5;x+=3) {
    let valid=true;
    for(let dy=0;dy<tile&&valid;dy++) for(let dx=0;dx<tile;dx++) if(mask[(y+dy)*width+x+dx]) {valid=false;break;}
    if(valid) {
      const values=[];
      for(let dy=0;dy<tile;dy++) for(let dx=0;dx<tile;dx++) {
        const at=((y+dy)*width+x+dx)*4;
        values.push((data[at]+data[at+1]+data[at+2])/3);
      }
      // Bevels and lettering have stronger contrast than paper grain.
      if(Math.max(...values)-Math.min(...values)<35) donors.push({x,y});
    }
  }
  // Narrow exposed rails may contain no clean six-pixel patch. Keep their
  // regional shading and use a flat residual instead of importing an edge.
  const hasGrain=donors.length>0;
  if(!hasGrain) donors.push({x:bounds.x,y:bounds.y});
  const chosen=new Map();
  const donor=(gx,gy)=>{
    const key=gy*cols*8+gx;
    if(chosen.has(key)) return chosen.get(key);
    let best=null,score=Infinity;
    for(let i=0;i<donors.length;i++) {
      const d=donors[i],distance=(d.x-gx*tile)**2+2*(d.y-gy*tile)**2;
      const jitter=((Math.imul(i+1,73856093)^Math.imul(gx+11,19349663)^Math.imul(gy+7,83492791))>>>0)%97;
      if(distance+jitter*4<score){best=d;score=distance+jitter*4;}
    }
    chosen.set(key,best);return best;
  };
  const output=new Uint8ClampedArray(data.length);
  for(let y=0;y<height;y++) for(let x=0;x<width;x++) {
    const at=(y*width+x)*4;
    if(!mask[y*width+x]) {output.set(data.subarray(at,at+4),at);continue;}
    const gx=Math.floor(x/tile),gy=Math.floor(y/tile),tx=(x%tile)/tile,ty=(y%tile)/tile;
    for(let c=0;c<3;c++) {
      let grain=0;
      // Overlap neighboring donor patches to avoid hard tile boundaries.
      for(let dy=0;dy<2;dy++) for(let dx=0;dx<2;dx++) {
        const d=donor(gx+dx,gy+dy);
        // A shared deterministic phase avoids repeating stripes from tiny donors.
        const phase=(Math.imul(x+1,374761393)^Math.imul(y+1,668265263))>>>0;
        const sx=d.x+phase%tile,sy=d.y+Math.floor(phase/tile)%tile;
        grain+=(hasGrain?1:0)*Math.max(-5,Math.min(5,data[(sy*width+sx)*4+c]-shade(sx,sy,c)))*(dx?tx:1-tx)*(dy?ty:1-ty);
      }
      output[at+c]=shade(x,y,c)+grain;
    }
    output[at+3]=255;
  }
  return {data:output,width,height,mask,cleanPixels:clean,donorCount:hasGrain?donors.length:0};
}
