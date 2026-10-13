import { MseError } from './document.mjs';
import { parseVectorSymbol, defaultVectorSymbol } from './vector-symbol.mjs';
import { symbolBounds, symbolAspectRatio, symbolShapePolygon } from './symbol-geometry.mjs';
import { filterSymbolRaster } from './symbol-variation.mjs';
import { allocateRaster } from './image-sampling.mjs';

// Shape rasterization uses browser polygon masks. Native pixel parity must be
// checked separately because wx and Canvas can disagree at polygon edges.
export function createBrowserSymbolRenderer({store,loadLocalSymbol,createCanvas=(w,h)=>new OffscreenCanvas(w,h)}={}) {
  return async (request,{width=0,height=0,maxPixels=16777216}={})=>{
    let symbol;
    if(!request.filename)symbol=defaultVectorSymbol();
    else if(request.local){
      if(!loadLocalSymbol)throw new MseError('symbol-local','A local set-symbol source provider is required');
      symbol=parseVectorSymbol(await loadLocalSymbol(request.filename),{file:request.filename});
    }else symbol=parseVectorSymbol(await store.readText(request.path),{file:request.path.key});
    if(!symbol.version)throw new MseError('symbol-version','Legacy symbol coordinate normalization is not implemented yet');
    const size=Math.max(100,3*Math.max(width,height)),smaller=width>1&&height>1;
    let w=smaller?Math.trunc(size*width/Math.max(width,height)):size;
    let h=smaller?Math.trunc(size*height/Math.max(width,height)):size;
    const aspect=symbolAspectRatio(symbolBounds(symbol)),ratio=w/h;
    if(ratio>aspect&&(aspect>1||(smaller&&h<w)))w=Math.trunc(h*aspect);
    else if(ratio<aspect&&(aspect<1||(smaller&&w<h)))h=Math.trunc(w/aspect);
    const out=allocateRaster(w,h,true,maxPixels),zoom=Math.max(w,h),ox=-(zoom-w)/2,oy=-(zoom-h)/2;
    const borderWidth=Math.trunc(request.variation.borderRadius*Math.min(w,h));
    const shapes=[];
    const visit=part=>{
      if(part.type==='group'){for(const p of [...part.parts].reverse())visit(p);return;}
      if(part.type!=='shape')throw new MseError('symbol-symmetry','Symmetry rasterization is not implemented yet');
      if(!['overlap','merge'].includes(part.combine))throw new MseError('symbol-combine',`Symbol combining is not implemented for ${part.combine}`);
      shapes.push(part);
    };
    for(const p of [...symbol.parts].reverse())visit(p);
    const canvas=createCanvas(w,h),ctx=canvas.getContext('2d',{willReadFrequently:true});
    const mask=(points,stroke)=>{
      ctx.clearRect(0,0,w,h);ctx.beginPath();
      points.forEach(([x,y],i)=>i?ctx.lineTo(x,y):ctx.moveTo(x,y));ctx.closePath();
      ctx.fillStyle='white';ctx.fill('evenodd');
      if(stroke&&borderWidth>0){ctx.lineWidth=borderWidth;ctx.lineJoin='round';ctx.strokeStyle='white';ctx.stroke();}
      return ctx.getImageData(0,0,w,h).data;
    };
    const border=new Uint8Array(w*h),inside=new Uint8Array(w*h);
    for(let i=0;i<w*h;i++)out.data.set([0,128,0,255],i*4);
    const flush=()=>{for(let i=0;i<w*h;i++){if(border[i])out.data.set([255,255,255,255],i*4);if(inside[i])out.data.set([0,0,0,255],i*4);}border.fill(0);inside.fill(0);};
    const round=v=>Math.trunc(v+(v>0?0.4999995:-0.4999995));
    for(const shape of shapes){
      if(shape.combine==='overlap')flush();
      const points=symbolShapePolygon(shape,{transform:([x,y])=>[round(ox+x*zoom),round(oy+y*zoom)]});
      if(points.length<3)continue;
      const fill=mask(points,false),outline=borderWidth>0?mask(points,true):null;
      for(let i=0;i<w*h;i++){if(fill[i*4+3]>=128)inside[i]=1;if(outline&&outline[i*4+3]>=128)border[i]=1;}
    }
    flush();return filterSymbolRaster(out,request.variation);
  };
}
