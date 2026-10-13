import {useEffect,useMemo,useRef,useState} from 'react';
import {useMseRenderer} from '@/context/MseRendererContext';
import {cardArtCropUrl} from '@/lib/card-image-variants';
import {selectMseStyle} from '@/lib/mse/style-selection.mjs';

// The existing frame stays visible while resources load or a render fails.
// A canvas is shown only when it belongs to the current printable snapshot.
export default function MseCardFrame({snapshot,imageUrl,style:requestedStyle,fallback}){
  const config=useMseRenderer(),canvas=useRef(null),[finished,setFinished]=useState(null);
  const session=config?.session;
  const card=useMemo(()=>JSON.parse(snapshot),[snapshot]);
  const style=selectMseStyle(card,{style:requestedStyle,defaultStyle:config?.defaultStyle});
  const request=useMemo(()=>({style,snapshot:card,adapter:{nativeCard:imageUrl?{image:cardArtCropUrl(imageUrl)}:{}}}),[card,imageUrl,style]);
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
    <canvas ref={canvas} data-mse-style={style} data-mse-frame={ready?'ready':finished?.request===request&&finished?.error?'error':'pending'}
      aria-hidden="true" style={{display:ready?'block':'none',width:'100%',height:'100%',objectFit:'contain'}} />
  </>;
}
