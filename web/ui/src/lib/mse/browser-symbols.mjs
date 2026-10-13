import { symbolDrawOrder } from './symbol-draw-order.mjs';
import { combineSymbolMasks } from './symbol-mask.mjs';
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
    const shapes=symbolDrawOrder(symbol);
    const canvas=createCanvas(w,h),ctx=canvas.getContext('2d',{willReadFrequently:true});
    const mask=(points,stroke,fill=true)=>{
      ctx.clearRect(0,0,w,h);ctx.beginPath();
      points.forEach(([x,y],i)=>i?ctx.lineTo(x,y):ctx.moveTo(x,y));ctx.closePath();
      ctx.fillStyle='white';if(fill)ctx.fill('evenodd');
      if(stroke&&borderWidth>0){ctx.lineWidth=borderWidth;ctx.lineJoin='round';ctx.strokeStyle='white';ctx.stroke();}
      return ctx.getImageData(0,0,w,h).data;
    };
    const border=new Uint8Array(w*h),inside=new Uint8Array(w*h);
    for(let i=0;i<w*h;i++)out.data.set([0,128,0,255],i*4);
    const flush=()=>{for(let i=0;i<w*h;i++){if(border[i])out.data.set([255,255,255,255],i*4);if(inside[i])out.data.set([0,0,0,255],i*4);}border.fill(0);inside.fill(0);};
    const round=v=>Math.trunc(v+(v>0?0.4999995:-0.4999995));
    for(const {shape,transform,allowOverlap} of shapes){
      if(shape.combine==='overlap'&&allowOverlap)flush();
      const points=symbolShapePolygon(shape,{transform:p=>{const [x,y]=transform(p);return [round(ox+x*zoom),round(oy+y*zoom)];}});
      if(points.length<3)continue;
      const fill=mask(points,false),outline=borderWidth>0?mask(points,true):null;
      const stroke=shape.combine==='subtract'&&borderWidth>0?mask(points,true,false):null;
      combineSymbolMasks(border,inside,fill,outline,stroke,shape.combine);
    }
    flush();return filterSymbolRaster(out,request.variation);
  };
}
