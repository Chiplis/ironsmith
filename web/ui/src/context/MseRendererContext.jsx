import {createContext,useContext,useEffect,useState} from 'react';
import {createBrowserMseSession} from '@/lib/mse/browser-session.mjs';

const MseRendererContext=createContext(null);
export const useMseRenderer=()=>useContext(MseRendererContext);

export function MseRendererProvider({corpusUrl,defaultStyle='auto',createSession=createBrowserMseSession,children}){
  const [loaded,setLoaded]=useState(null);
  useEffect(()=>{
    if(!corpusUrl)return undefined;
    const controller=new AbortController();let session;
    createSession({corpusUrl,signal:controller.signal}).then(value=>{
      if(controller.signal.aborted){value.dispose();return;}
      session=value;setLoaded({corpusUrl,createSession,session:value});
    }).catch(error=>{
      if(!controller.signal.aborted)setLoaded({corpusUrl,createSession,error});
    });
    return ()=>{controller.abort();session?.dispose();};
  },[corpusUrl,createSession]);
  const current=loaded?.corpusUrl===corpusUrl&&loaded?.createSession===createSession?loaded:null;
  return <MseRendererContext.Provider value={corpusUrl?{...current,defaultStyle}:null}>{children}</MseRendererContext.Provider>;
}
