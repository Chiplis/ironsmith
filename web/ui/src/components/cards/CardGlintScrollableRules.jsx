import {useEffect,useRef,useState} from 'react';
import {renderCardGlintRulesBackdrop} from '@/lib/card-glint/session.mjs';
import CardFrameRules from './CardFrameRules';
import '@/styles/card-typography.css';

export default function CardGlintScrollableRules({result,rulesView,onActivate,highlighted,name}) {
  const canvas=useRef(null),scroll=useRef(null);
  const [painted,setPainted]=useState(null);
  useEffect(()=>{
    let cancelled=false;
    renderCardGlintRulesBackdrop(result).then(backdrop=>{
      if(cancelled)return;
      const target=canvas.current;
      target.width=backdrop.width;target.height=backdrop.height;
      target.getContext('2d').drawImage(backdrop,0,0);
      scroll.current.scrollTop=0;
      setPainted(result);
    }).catch(error=>console.error('[card-glint] Rules backdrop failed',error));
    return()=>{cancelled=true;};
  },[result]);
  const {api,face}=result,rects=api.faceRects(face);
  const walker=api.faceFrame(face).startsWith('planeswalker');
  const [x,y,w,h]=rects.rules;
  const inset=walker?40/488:0;
  // Keep the curved lower border and the printed stats outside the scrollbar.
  const bottom=face.loyalty!=null||face.power!=null||face.defense!=null
    ? Math.min(y+h,rects.stats[1]) : y+h;
  const ready=painted===result;
  return <div className="card-glint-scrollable" style={{visibility:ready?'visible':'hidden'}}>
    <canvas ref={canvas} aria-hidden="true"/>
    <div ref={scroll} className="card-glint-scrollable__rules" aria-label={`${name} abilities`}
      style={{left:`${(x+inset)*100}%`,top:`${(y+8/680)*100}%`,width:`${(w-inset)*100}%`,height:`${(bottom-y-12/680)*100}%`}}>
      <CardFrameRules name={name} rulesView={rulesView} onActivate={onActivate} highlighted={highlighted}/>
      {face.flavorText&&<p className="simple-card-frame__flavor">{face.flavorText}</p>}
    </div>
  </div>;
}
