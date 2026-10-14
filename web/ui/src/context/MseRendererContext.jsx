import {createContext,useContext,useEffect,useRef,useState} from 'react';
import {createBrowserMseSession} from '@/lib/mse/browser-session.mjs';
import {MseDefaultFrames} from '@/lib/mse/default-frames.mjs';

const MseRendererContext=createContext(null);
export const useMseRenderer=()=>useContext(MseRendererContext);

export function MseRendererProvider({corpusUrl,defaultStyle='auto',createSession=createBrowserMseSession,children}){
  const [loaded,setLoaded]=useState(null),defaultStyleRef=useRef(defaultStyle);
  useEffect(()=>{defaultStyleRef.current=defaultStyle;},[defaultStyle]);
  useEffect(()=>{
    if(!corpusUrl)return undefined;
    const controller=new AbortController();let session,defaultFrames;
    createSession({corpusUrl,signal:controller.signal}).then(value=>{
      if(controller.signal.aborted){value.dispose();return;}
      // Blank per-color frames stand in for cards whose own render is pending.
      if(value.renderer?.render){
        defaultFrames=new MseDefaultFrames({renderer:value.renderer,fonts:value.fonts,defaultStyle:defaultStyleRef.current,availableStyles:value.styles});
        defaultFrames.warm();
      }
      session=value;setLoaded({corpusUrl,createSession,session:value,defaultFrames});
    }).catch(error=>{
      if(!controller.signal.aborted){console.error('[MSE] Corpus initialization failed', {corpusUrl,error});setLoaded({corpusUrl,createSession,error});}
    });
    return ()=>{controller.abort();defaultFrames?.dispose();session?.dispose();};
  },[corpusUrl,createSession]);
  const current=loaded?.corpusUrl===corpusUrl&&loaded?.createSession===createSession?loaded:null;
  return <MseRendererContext.Provider value={corpusUrl?{...current,defaultStyle}:null}>{children}</MseRendererContext.Provider>;
}
