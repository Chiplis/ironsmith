import {useEffect,useMemo,useRef,useState} from 'react';
import {useMseRenderer} from '@/context/MseRendererContext';
import {cardArtCropUrl} from '@/lib/card-image-variants';
import {selectMsePresentation} from '@/lib/mse/style-selection.mjs';
import {printingMseCardFields} from '@/lib/mse/printing-style.mjs';
import {resolveScryfallPrintingMetadata} from '@/lib/scryfall';

// The existing frame stays visible while resources load or a render fails.
// A canvas is shown only when it belongs to the current printable snapshot.
export default function MseCardFrame({snapshot,imageUrl,style:requestedStyle,faceIndex=0,fallback}){
  const config=useMseRenderer(),canvas=useRef(null),[finished,setFinished]=useState(null);
  const [resolvedPrinting,setResolvedPrinting]=useState(null);
  const session=config?.session;
  const card=useMemo(()=>JSON.parse(snapshot),[snapshot]);
  const explicitStyle=(requestedStyle&&requestedStyle!=='auto')||(config?.defaultStyle&&config.defaultStyle!=='auto');
  useEffect(()=>{
    if(!session||!imageUrl||explicitStyle)return undefined;
    let cancelled=false;
    resolveScryfallPrintingMetadata(imageUrl).then(printing=>{
      if(!cancelled)setResolvedPrinting({imageUrl,printing});
    }).catch(()=>{if(!cancelled)setResolvedPrinting({imageUrl,printing:null});});
    return ()=>{cancelled=true;};
  },[session,imageUrl,explicitStyle]);
  const printing=resolvedPrinting&&resolvedPrinting.imageUrl===imageUrl?resolvedPrinting.printing:null;
  const presentation=useMemo(()=>selectMsePresentation(card,{style:requestedStyle,defaultStyle:config?.defaultStyle,faceIndex,
    printing,availableStyles:session?.styles}),[card,requestedStyle,config?.defaultStyle,faceIndex,printing,session]);
  const {style,styling}=presentation;
  const request=useMemo(()=>({style,snapshot:card,adapter:{styling,faceIndex:card.card_faces&&!['split','adventure'].includes(card.layout)?faceIndex:null,
    nativeCard:{...(!explicitStyle?printingMseCardFields(card,printing):{}),...(imageUrl?{image:cardArtCropUrl(imageUrl)}:{})}}}),[card,imageUrl,style,styling,faceIndex,explicitStyle,printing]);
  useEffect(()=>{
    if(!session||!style)return undefined;
    const controller=new AbortController(),target=canvas.current;
    session.renderer.renderTo(target,request,{signal:controller.signal}).then(()=>{
      if(!controller.signal.aborted)setFinished({request,session});
    }).catch(error=>{
      if(!controller.signal.aborted)setFinished({request,session,error});
    });
    return ()=>controller.abort();
  },[request,session,style]);
  const ready=finished?.request===request&&finished?.session===session&&!finished?.error;
  return <>
    {!ready&&fallback}
    <canvas ref={canvas} data-mse-style={style} data-mse-selection={presentation.source} data-mse-frame={ready?'ready':session&&!style?'unavailable':finished?.request===request&&finished?.error?'error':'pending'}
      aria-hidden="true" style={{display:ready?'block':'none',width:'100%',height:'100%',objectFit:'contain'}} />
  </>;
}
