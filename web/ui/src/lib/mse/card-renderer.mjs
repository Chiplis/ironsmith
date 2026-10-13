import {MseError} from './document.mjs';
import {loadMseTemplate,MseTemplateContext} from './template.mjs';
import {paintMseCard} from './card-painter.mjs';
import {engineCardToMse} from './engine-card.mjs';
import {MseImageRenderer} from './image-renderer.mjs';
import {createMseImageLoader} from './browser-images.mjs';
import {createBrowserSymbolRenderer} from './browser-symbols.mjs';

// Resource providers are owned by the caller so multiple card surfaces can
// share registered fonts and decoded assets without disposing one another.
export class MseCardRenderer{
  constructor({store,fonts,imageRenderer,createCanvas=(w,h)=>new OffscreenCanvas(w,h),loadCardImage}={}){
    this.store=store;this.fonts=fonts;this.createCanvas=createCanvas;this.loadCardImage=loadCardImage;
    this.images=imageRenderer??new MseImageRenderer({loadAsset:createMseImageLoader(store),renderSymbol:createBrowserSymbolRenderer({store})});
    this.templates=new Map();this.targets=new WeakMap();this.disposed=false;
  }
  check(signal){
    if(this.disposed)throw new MseError('renderer-disposed','Card renderer has been disposed');
    if(signal?.aborted)throw new MseError('render-aborted','Card rendering was cancelled');
  }
  async template(style){
    this.check();
    if(!this.templates.has(style)){
      const pending=loadMseTemplate(this.store,style).catch(error=>{if(this.templates.get(style)===pending)this.templates.delete(style);throw error;});
      this.templates.set(style,pending);
    }
    const template=await this.templates.get(style);this.check();return template;
  }
  async render({style,context={},snapshot,adapter={}},{signal,onFieldDrawn}={}){
    this.check(signal);
    const template=await this.template(style);this.check(signal);
    const values=snapshot===undefined?context:engineCardToMse(snapshot,adapter);
    const card=new MseTemplateContext(template,{...values,spellcheckEnabled:false}).initialize();
    return this.renderContext(card,{signal,onFieldDrawn});
  }
  async renderContext(context,{signal,onFieldDrawn}={}){
    this.check(signal);
    const width=Number(context.template.style.text('card width')),height=Number(context.template.style.text('card height'));
    if(!Number.isSafeInteger(width)||!Number.isSafeInteger(height)||width<1||height<1||width*height>(this.images.maxPixels??16777216))throw new MseError('card-size','Invalid card canvas dimensions');
    const canvas=this.createCanvas(width,height);
    const result=await paintMseCard(canvas.getContext('2d'),context,this.fonts,this.images,{
      createCanvas:this.createCanvas,loadCardImage:this.loadCardImage,
      onFieldDrawn:async info=>{this.check(signal);if(onFieldDrawn)await onFieldDrawn(info);},
    });
    this.check(signal);
    return {...result,canvas,context};
  }
  async renderTo(target,request,options={}){
    this.check(options.signal);
    const token={};this.targets.set(target,token);
    const result=await this.render(request,options);
    this.check(options.signal);
    if(this.targets.get(target)!==token)throw new MseError('render-superseded','A newer card render replaced this request');
    // Allocate and draw privately before touching the visible canvas. Failed,
    // cancelled, or superseded work leaves the current card intact.
    target.width=result.width;target.height=result.height;
    target.getContext('2d').drawImage(result.canvas,0,0);
    return result;
  }
  dispose(){this.disposed=true;this.templates.clear();this.targets=new WeakMap();}
}
