import {MANA_SYMBOL_SVGS} from './mana-symbol-svg.js';
const templates=new Map();
// Match the same symbol artwork as the visible renderer. Hybrid/Phyrexian
// costs are not in the small embedded SVG table, but still need mask templates.
export function manaTemplateSource(key) {
  const svg=MANA_SYMBOL_SVGS[key];
  if(svg)return `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="${svg.vb}" width="48" height="48">${svg.html}</svg>`)}`;
  if (/^(?:[WUBRG]\/P|[WUBRGC2]\/[WUBRG]|[WUBRG]\/[WUBRG]\/P)$/.test(key)) {
    return `https://svgs.scryfall.io/card-symbols/${key.replaceAll('/', '')}.svg`;
  }
  return null;
}
// The rules text renders the same CDN symbols as plain <img> elements, whose
// requests carry no Origin header, so the CDN answers them without CORS
// headers. Chromium then reuses that cached response for a CORS request to the
// same URL and the template fails to load, exactly for the symbols the card
// displays. A distinct URL keeps the template out of that cache entry, and a
// blob URL is same-origin for the canvas.
async function loadTemplateImage(source) {
  let url=source,revoke=null;
  if(!source.startsWith('data:')) {
    const response=await fetch(`${source}${source.includes('?')?'&':'?'}template`,{mode:'cors',referrerPolicy:'no-referrer'});
    if(!response.ok)throw new Error(`Mana symbol unavailable: ${response.status}`);
    url=URL.createObjectURL(await response.blob());revoke=url;
  }
  try {
    const image=new Image();
    await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=()=>reject(new Error('Mana symbol failed to load'));image.src=url;});
    return image;
  } finally {if(revoke)URL.revokeObjectURL(revoke);}
}
export async function manaTemplates(cost) {
  const keys=[...String(cost||'').matchAll(/\{([^}]+)\}/g)].map(m=>m[1].toUpperCase());
  return Promise.all(keys.map(async key=>{
    if(!templates.has(key))templates.set(key,(async()=>{
      const source=manaTemplateSource(key);if(!source)return null;
      const image=await loadTemplateImage(source);
      const canvas=document.createElement('canvas');canvas.width=48;canvas.height=48;
      const ctx=canvas.getContext('2d',{willReadFrequently:true});ctx.drawImage(image,0,0,48,48);
      return {key,data:ctx.getImageData(0,0,48,48).data,width:48,height:48};
    })().catch(error=>{templates.delete(key);throw error;}));
    return templates.get(key);
  }));
}

// Fit the ordered SVG glyphs as one row. Correlation ignores paper/rarity
// color and also supports the reversed ink of retro black mana symbols.
export function locateManaSymbols(scan,box,icons,{vertical=false,bounds=null}={}) {
  if(!icons.length||icons.some(t=>!t))return null;
  const samples=icons.map(icon=>{
    const points=[];
    for(let gy=0;gy<12;gy++)for(let gx=0;gx<12;gx++) {
      const u=(gx+.5)/12,v=(gy+.5)/12;
      if(Math.hypot(u-.5,v-.5)>.43)continue;
      const p=(Math.floor(v*icon.height)*icon.width+Math.floor(u*icon.width))*4;
      points.push({u,v,value:(icon.data[p]+icon.data[p+1]+icon.data[p+2])/3});
    }
    const mean=points.reduce((s,p)=>s+p.value,0)/points.length;
    let norm=0;for(const p of points){p.value-=mean;norm+=p.value*p.value;}
    return {points,norm};
  });
  const correlation=(t,x,y,d)=>{
    let sum=0,squares=0,dot=0;
    for(const p of t.points) {
      const at=(Math.round(y+p.v*d)*scan.width+Math.round(x+p.u*d))*4;
      const value=(scan.data[at]+scan.data[at+1]+scan.data[at+2])/3;
      sum+=value;squares+=value*value;dot+=value*p.value;
    }
    const variance=squares-sum*sum/t.points.length;
    return variance>t.points.length*80?Math.abs(dot)/Math.sqrt(variance*t.norm):0;
  };
  if(vertical && bounds) {
    // Future Sight costs follow a curve: each disc has its own horizontal
    // position. Fit their ordered vertical spacing without forcing one x.
    const rows=new Map();
    const row=(index,y,d)=>{
      const key=`${index}|${y}|${d}`;
      if(rows.has(key))return rows.get(key);
      let found=null;
      for(let x=Math.ceil(bounds.x);x<=bounds.x+bounds.width-d;x++) {
        const score=correlation(samples[index],x,y,d);
        if(!found || score>found.score)found={x,y,width:d,height:d,key:icons[index].key,score};
      }
      rows.set(key,found);return found;
    };
    let column=null;
    for(let d=14;d<=32;d+=2)for(let gap=1;gap<=13;gap+=2) {
      const span=icons.length*d+(icons.length-1)*gap;
      const bottom=Math.min(bounds.y+bounds.height-span,bounds.y+scan.height*.07);
      for(let y=Math.ceil(bounds.y);y<=bottom;y+=2) {
        const symbols=icons.map((_,index)=>row(index,y+index*(d+gap),d));
        if(symbols.some(s=>!s||s.score<.48))continue;
        const confidence=symbols.reduce((sum,s)=>sum+s.score,0)/symbols.length;
        if(!column || confidence>column.confidence)column={confidence,symbols};
      }
    }
    if(!column)return null;
    const symbols=column.symbols.map((coarse,index)=>{
      let best=coarse;
      for(let d=coarse.width-1;d<=coarse.width+1;d++)for(let y=coarse.y-1;y<=coarse.y+1;y++)for(let x=coarse.x-2;x<=coarse.x+2;x++) {
        const score=correlation(samples[index],x,y,d);
        if(score>best.score)best={...coarse,x,y,width:d,height:d,score};
      }
      const {score: _score,...symbol}=best;return symbol;
    });
    return {confidence:column.confidence,symbols};
  }
  let best=null;
  const positions=(x,y,d,gap)=>samples.map((t,i)=>correlation(t,x+(vertical?0:i*(d+gap)),y+(vertical?i*(d+gap):0),d));
  const consider=(x,y,d,gap)=>{
    const scores=positions(x,y,d,gap),score=scores.reduce((s,v)=>s+v,0)/scores.length;
    if(Math.min(...scores)>.25&&score>.48&&(!best||score>best.confidence))best={x,y,d,gap,confidence:score};
  };
  for(let d=14;d<=Math.min(32,box.height+6);d+=2)for(let gap=1;gap<= (vertical?12:5);gap+=2) {
    const span=icons.length*d+(icons.length-1)*gap;
    if(bounds) {
      for(let x=Math.ceil(bounds.x);x<=bounds.x+bounds.width-(vertical?d:span);x+=2)
        for(let y=Math.ceil(bounds.y);y<=bounds.y+bounds.height-(vertical?span:d);y+=2)consider(x,y,d,gap);
    } else {
      for(let right=Math.floor(scan.width*.9);right<=scan.width*.942;right+=2) {
        const x=right-span;if(x<scan.width*.52)continue;
        for(let y=Math.max(1,Math.floor(box.y-3));y<=box.y+box.height-d+5;y+=2)consider(x,y,d,gap);
      }
    }
  }
  if(!best)return null;
  // Refine translation and scale around the coarse registration.
  const coarse=best;
  for(let d=coarse.d-1;d<=coarse.d+1;d++)for(let gap=Math.max(0,coarse.gap-1);gap<=coarse.gap+1;gap++)for(let y=coarse.y-1;y<=coarse.y+1;y++)for(let x=coarse.x-2;x<=coarse.x+2;x++) {
    const scores=positions(x,y,d,gap);
    const score=scores.reduce((s,v)=>s+v,0)/scores.length;
    if(Math.min(...scores)>.25&&score>best.confidence)best={x,y,d,gap,confidence:score};
  }
  return {confidence:best.confidence,symbols:icons.map((t,i)=>({key:t.key,x:best.x+(vertical?0:i*(best.d+best.gap)),y:best.y+(vertical?i*(best.d+best.gap):0),width:best.d,height:best.d}))};
}
