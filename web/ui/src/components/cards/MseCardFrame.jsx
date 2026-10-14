import {useEffect,useMemo,useRef,useState,useSyncExternalStore} from 'react';
import {useMseRenderer} from '@/context/MseRendererContext';
import {mseFrameRequest} from '@/lib/mse/frame-request.mjs';
import {peekScryfallPrintingMetadata,peekScryfallSetIcon,resolveScryfallPrintingMetadata,resolveScryfallSetIcon} from '@/lib/scryfall';
import MseCardInteractions from './MseCardInteractions';
import MseStandIn from './MseStandIn';
import {mseInteractionRegions,mseActionsCovered} from '@/lib/mse/interaction-regions.mjs';
import {mseDefaultFrameKey} from '@/lib/mse/default-frames.mjs';

// A warm render takes tens of milliseconds. While a different card renders it
// shows the blank MSE frame for its colors; the simple fallback appears only
// when no such frame exists and the grace period passes, or the render fails.
const FALLBACK_GRACE_MS=200;
const noFrames=()=>()=>{};
const cardIdentity=request=>JSON.stringify([request.style,request.snapshot.name,request.adapter.faceIndex]);

// The existing frame stays visible while resources load or a render fails.
// Interactions belong only to the canvas of the current printable snapshot;
// a re-render of the same card keeps its previous canvas on screen meanwhile.
export default function MseCardFrame({snapshot,imageUrl,style:requestedStyle,faceIndex=0,fallback,rulesView,onActivate,highlighted,details}){
  const config=useMseRenderer(),canvas=useRef(null),[finished,setFinished]=useState(null),[overdue,setOverdue]=useState(null);
  // The request whose stand-in reached the screen, and the one whose fade ended.
  const [shown,setShown]=useState(null),[dismissed,setDismissed]=useState(null);
  const [resolvedPrinting,setResolvedPrinting]=useState(null);
  const session=config?.session;
  const card=useMemo(()=>JSON.parse(snapshot),[snapshot]);
  // Printing metadata and set icons already fetched (e.g. by background
  // preparation) are read synchronously, so the first request is complete
  // and the frame renders once instead of restarting as each one arrives.
  const printing=useMemo(()=>resolvedPrinting&&resolvedPrinting.imageUrl===imageUrl?resolvedPrinting.printing:peekScryfallPrintingMetadata(imageUrl)??null,
    [resolvedPrinting,imageUrl]);
  useEffect(()=>{
    // The footer credits the displayed printing even when a style is forced.
    if(!session||!imageUrl||peekScryfallPrintingMetadata(imageUrl)!==undefined)return undefined;
    let cancelled=false;
    resolveScryfallPrintingMetadata(imageUrl).then(value=>{
      if(!cancelled)setResolvedPrinting({imageUrl,printing:value});
    }).catch(()=>{if(!cancelled)setResolvedPrinting({imageUrl,printing:null});});
    return ()=>{cancelled=true;};
  },[session,imageUrl]);
  // The set symbol is the printing's set icon, colored by the template's rarity variations.
  const setCode=String((printing??card.printing??card).set??'').toLowerCase(),[setIcon,setSetIcon]=useState(null);
  useEffect(()=>{
    if(!session||!setCode||peekScryfallSetIcon(setCode)!==undefined)return undefined;
    let cancelled=false;
    resolveScryfallSetIcon(setCode).then(url=>{if(!cancelled)setSetIcon({setCode,url});});
    return ()=>{cancelled=true;};
  },[session,setCode]);
  const setSymbol=(setIcon?.setCode===setCode?setIcon.url:peekScryfallSetIcon(setCode))??null;
  const {presentation,request}=useMemo(()=>mseFrameRequest(card,{imageUrl,requestedStyle,defaultStyle:config?.defaultStyle,faceIndex,
    printing,setSymbol,availableStyles:session?.styles}),[card,imageUrl,requestedStyle,config?.defaultStyle,faceIndex,printing,setSymbol,session]);
  const {style}=presentation;
  const frames=config?.defaultFrames,frameKey=useMemo(()=>mseDefaultFrameKey(card,faceIndex),[card,faceIndex]);
  const placeholder=useSyncExternalStore(frames?.subscribe??noFrames,()=>frames?.lookup(style,frameKey)??null);
  useEffect(()=>{frames?.request(style,frameKey);},[frames,style,frameKey]);
  useEffect(()=>{
    if(!session||!style)return undefined;
    const timer=setTimeout(()=>setOverdue(request),FALLBACK_GRACE_MS);
    return ()=>clearTimeout(timer);
  },[request,session,style]);
  useEffect(()=>{
    if(!session||!style)return undefined;
    const controller=new AbortController(),target=canvas.current;
    session.renderer.renderTo(target,request,{signal:controller.signal}).then(result=>{
      if(!controller.signal.aborted)setFinished({request,session,result});
    }).catch(error=>{
      if(!controller.signal.aborted){console.error('[MSE] Card render failed', {name:request.snapshot.name,style,error});setFinished({request,session,error});}
    });
    return ()=>controller.abort();
  },[request,session,style]);
  const actionsCovered=useMemo(()=>mseActionsCovered(mseInteractionRegions(finished?.result,rulesView),rulesView),[finished,rulesView]);
  const rendered=finished?.request===request&&finished?.session===session&&!finished?.error;
  const ready=rendered&&actionsCovered;
  const error=config?.error||(finished?.request===request?finished?.error:null);
  const status=ready?'ready':error?'error':!config?'disabled':(session&&!style)||(rendered&&!actionsCovered)?'unavailable':'pending';
  // The canvas still holds the last finished render until a new one commits.
  const stale=status==='pending'&&finished&&!finished.error&&finished.session===session&&finished.request!==request
    &&cardIdentity(finished.request)===cardIdentity(request);
  // Corpus loading has no grace period: the session may take seconds.
  const waiting=status==='pending'&&Boolean(session&&style)&&overdue!==request;
  const pendingStandIn=status==='pending'&&!stale&&Boolean(placeholder);
  // A stand-in that was on screen fades out over the finished canvas.
  const leavingStandIn=ready&&Boolean(placeholder)&&shown===request&&dismissed!==request;
  const surface=ready||stale||pendingStandIn||leavingStandIn;
  const width=ready||stale?finished?.result?.width||375:placeholder?.width||375;
  const height=ready||stale?finished?.result?.height||523:placeholder?.height||523;
  return <>
    {!ready&&!stale&&!pendingStandIn&&!waiting&&fallback}
    <div className="mse-card-surface" style={{display:surface?'block':'none',width:`min(100cqw,${100*width/height}cqh)`,height:`min(100cqh,${100*height/width}cqw)`}}>
      <canvas ref={canvas} data-mse-style={style} data-mse-selection={presentation.source} data-mse-frame={status}
        data-mse-error={error?.message||(rendered&&!actionsCovered?'No measured text region for an available ability':undefined)} aria-hidden="true"
        style={{width:'100%',height:'100%',visibility:ready||stale?'visible':'hidden'}} />
      {(pendingStandIn||leavingStandIn)&&<MseStandIn key={placeholder.url} frame={placeholder} card={card} imageUrl={imageUrl} faceIndex={faceIndex}
        placeholder={`${frameKey.colors||'C'}${frameKey.kind?`-${frameKey.kind}`:''}`}
        leaving={leavingStandIn} onShown={()=>setShown(request)} onLeft={()=>setDismissed(request)}/>}
      {ready&&rulesView&&<MseCardInteractions result={finished.result} rulesView={rulesView} onActivate={onActivate} highlighted={highlighted} name={card.name}/>}
      {details&&<span className="mse-card-live-details">{details}</span>}
    </div>
  </>;
}
