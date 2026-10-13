import { paintChoiceLabel } from './choice-painter.mjs';
import { MseError } from './document.mjs';
import { text, force } from './script-values.mjs';
import { paintTextLayout } from './text-painter.mjs';
import { parseAlignment, alignmentOffset } from './alignment.mjs';
import { cssColor } from './symbol-painter.mjs';
import { color } from './color-values.mjs';
import { mseImage } from './image-builtins.mjs';
import { resizeRaster, allocateRaster } from './image-sampling.mjs';
import { resizeArtwork } from './artwork-sampling.mjs';
import { combineChannel } from './image-combine.mjs';
import { drawCombinedLayer } from './field-blend.mjs';

// Draw into a private surface so a failed field never commits a partial card.
// Unsupported drawing paths remain explicit until their native behavior exists.
export async function paintMseCard(target,context,fonts,renderer,{createCanvas=(w,h)=>new OffscreenCanvas(w,h),loadCardImage,onFieldDrawn}={}) {
  await context.prepareTextLayouts(fonts,renderer);
  const revision=context.layoutRevision;
  const width=Number(context.template.style.text('card width')),height=Number(context.template.style.text('card height'));
  if(!Number.isSafeInteger(width)||!Number.isSafeInteger(height)||width<1||height<1||width*height>(renderer.maxPixels||16777216))throw new MseError('card-size','Invalid card canvas dimensions');
  const surface=createCanvas(width,height),base=surface.getContext('2d');
  base.fillStyle=cssColor(color(context.template.style.text('card background','white')));base.fillRect(0,0,width,height);
  const fields=[];
  for(const [group,layouts] of [['card',context.template.layouts],['extra_card',context.template.extraLayouts]])for(const [name,layout] of layouts){
    if(context.fieldVisible(name,group))fields.push({group,name,layout,z:layout.programs.has('z_index')?context.property(name,'z_index',group):0});
  }
  fields.sort((a,b)=>a.z-b.z);
  for(const {group,name,layout} of fields){
    try{
      const get=(key,fallback)=>layout.programs.has(key)?context.property(name,key,group):fallback;
      const type=context.cells[group].get(name)?.definition?.record.text('type');
      const left=context.property(name,'left',group),top=context.property(name,'top',group);
      const w=context.property(name,'width',group),h=context.property(name,'height',group);
      const angle=Number(context.property(name,'angle',group));
      if(!Number.isFinite(angle))throw new MseError('card-rotation','Field angle must be finite');
      // Native text/art viewers do not have a combine property. A color
      // field's combine mode applies only to its mask-generated bitmap.
      const supportsCombine=['choice','boolean','multiple choice'].includes(type)||(type==='color'&&get('mask',null));
      let combine=supportsCombine?text(get('combine','normal')):'normal';
      if(['choice','boolean','multiple choice'].includes(type)){
        const imageMode=context.textMeasurements.get(`${group}.${name}`)?.result?.combine;
        if(imageMode&&imageMode!=='default')combine=imageMode;
      }
      if(combine===''||combine==='default')combine='normal';
      combineChannel(combine);
      const layer=combine==='normal'?null:createCanvas(width,height);
      const ctx=layer?layer.getContext('2d'):base;
      let label=null;
      ctx.save();
      try{
        ctx.translate(left,top);
        // MSE's Rotation::tr uses (cos*x + sin*y, -sin*x + cos*y).
        // Canvas angles have the opposite sign in the same downward Y axis.
        if(angle%360)ctx.rotate(-(angle%360)*Math.PI/180);
        if(type==='text'){
          const result=context.textMeasurements.get(`${group}.${name}`)?.result;
          if(result)await paintTextLayout(ctx,result,fonts);
        }else{
          let image;
          if(['choice','boolean','multiple choice'].includes(type)){
            const style=text(get('render_style','text'));
            const value=text(context.field(group,name));
            if(value&&!style.includes('hidden')){
              image=style.includes('image')?context.textMeasurements.get(`${group}.${name}`)?.result?.image:null;
              if(style.includes('text'))label={value,font:context.font(name,group),options:{width:w,height:h,alignment:text(get('alignment','stretch')),withImage:style.includes('image'),imageWidth:image?.width??null}};
            }
            const maskValue=get('mask',null);
            if(image && maskValue!==null && maskValue!==''){
              const mask=resizeRaster(await renderer.render(mseImage(maskValue,context.template.name),{width:image.width,height:image.height}),image.width,image.height,renderer.maxPixels);
              image={...image,data:image.data.slice(),hasAlpha:true};
              for(let i=3;i<image.data.length;i+=4)image.data[i]=Math.trunc(image.data[i]*mask.data[i-3]/255);
            }
          }else if(type==='color'){
            const value=color(context.field(group,name)),maskValue=get('mask',null);
            if(maskValue!==null && maskValue!==''){
              const iw=Math.max(1,Math.trunc(w)),ih=Math.max(1,Math.trunc(h));
              const mask=resizeRaster(await renderer.render(mseImage(maskValue,context.template.name),{width:iw,height:ih}),iw,ih,renderer.maxPixels);
              image=allocateRaster(iw,ih,true,renderer.maxPixels);
              for(let i=0;i<image.data.length;i+=4){
                image.data.set(value.channels,i);
                // Native colorImage fills RGB without an alpha plane, then
                // installs the mask alpha (the color's alpha is ignored).
                image.data[i+3]=mask.data[i];
              }
            }else{
              const l=Number(get('left_width',100000)),r=Number(get('right_width',100000));
              const t=Number(get('top_width',100000)),b=Number(get('bottom_width',100000));
              if(l<w && r<w && t<h && b<h){
                ctx.beginPath();ctx.rect(0,0,w,h);
                if(w-l-r>0 && h-t-b>0)ctx.rect(l,t,w-l-r,h-t-b);
                ctx.clip('evenodd');
              }
              const radius=Number(get('radius',0));
              ctx.fillStyle=cssColor(value);ctx.beginPath();
              ctx.roundRect(0,0,w,h,radius<0?-radius*Math.min(w,h):radius);ctx.fill();
            }
          }else if(type==='image'){
            const value=force(context.field(group,name));
            const iw=Math.max(1,Math.trunc(w)),ih=Math.max(1,Math.trunc(h));
            if(value!==null&&value!==''){
              if(!loadCardImage)throw new MseError('card-art','A card artwork provider is required');
              image=await loadCardImage(value,{width:iw,height:ih,field:name,group});
              if(image)image=resizeArtwork(image,iw,ih,renderer.maxPixels);
            }
            const defaultValue=get('default',null);
            if(!image&&defaultValue!==null&&defaultValue!==''){
              const graph=mseImage(defaultValue,context.template.name);
              if(graph.operation!=='blank')image=resizeRaster(await renderer.render(graph,{width:iw,height:ih}),iw,ih,renderer.maxPixels);
            }
            const maskValue=get('mask',null);
            if(image&&maskValue!==null&&maskValue!==''){
              const mask=resizeRaster(await renderer.render(mseImage(maskValue,context.template.name),{width:iw,height:ih}),iw,ih,renderer.maxPixels);
              image={...image,data:image.data.slice(),hasAlpha:true};
              for(let i=3;i<image.data.length;i+=4)image.data[i]=Math.trunc(image.data[i]*mask.data[i-3]/255);
            }
          }else throw new MseError('card-field',`Drawing is not implemented for field type ${type}`);
          if(image){
            const bitmap=createCanvas(image.width,image.height),dc=bitmap.getContext('2d'),pixels=dc.createImageData(image.width,image.height);
            pixels.data.set(image.data);dc.putImageData(pixels,0,0);
            const offset=alignmentOffset(parseAlignment(text(get('alignment',type==='color'?'left top':'stretch'))),w,h,image.width,image.height);
            ctx.drawImage(bitmap,offset.x,offset.y);
          }
        }
      }finally{ctx.restore();}
      if(layer)drawCombinedLayer(base,layer,combine,createCanvas);
      // Native combine modes affect the choice image, not its label.
      if(label){
        base.save();
        try{base.translate(left,top);if(angle%360)base.rotate(-(angle%360)*Math.PI/180);
          await paintChoiceLabel(base,label.value,label.font,fonts,label.options);
        }finally{base.restore();}
      }
      if(onFieldDrawn)await onFieldDrawn({group,name,surface});
    }catch(error){error.field=`${group}.${name}`;throw error;}
  }
  if(revision!==context.layoutRevision)throw new MseError('layout-stale','Card changed while it was being drawn');
  target.drawImage(surface,0,0);
  return {width,height,fields:fields.length,nativeParityVerified:false};
}
