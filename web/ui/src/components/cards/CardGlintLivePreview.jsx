import {useEffect,useMemo,useRef,useState} from 'react';
import {loadCardGlint,renderCardGlint} from '@/lib/card-glint/session.mjs';
import useCardGlintLive from './useCardGlintLive';
import './card-frame-presentation.css';

// Border rails sit outside the printed frame, as on the physical card.
const BORDER_RAILS = [[3.5,0,93,3],[3.5,97,93,3],[0,3,3.5,94],[96.5,3,3.5,94]];
const percent = value => `${value*100}%`;

function useSettled(value,delay) {
  const [settled,setSettled] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setSettled(value),delay);
    return () => clearTimeout(timer);
  },[value,delay]);
  return settled;
}

// Keep the preview face rectangular; only the finish follows the pointer.
export default function CardGlintLivePreview({face,presentation,onBorderClick,onRarityClick,labels={},liveEnabled=true}) {
  const still = useRef(null), stage = useRef(null);
  const [api,setApi] = useState(null), [stillKey,setStillKey] = useState(null), [stillFailed,setStillFailed] = useState(false);
  const key = useMemo(() => face ? JSON.stringify(face) : '', [face]);
  // Prepare only the settled draft, not every keystroke.
  const settledKey = useSettled(key,140);
  const settled = useMemo(() => settledKey ? JSON.parse(settledKey) : null, [settledKey]);
  const [liveCanvas,live] = useCardGlintLive(settled,presentation,{enabled:liveEnabled});
  const fallback = live.mode === 'unavailable' || live.mode === 'off';

  useEffect(() => {
    let cancelled = false;
    loadCardGlint().then(value => {if (!cancelled) setApi(value);}).catch(() => {});
    return () => {cancelled = true;};
  },[]);

  useEffect(() => {
    if (!fallback || !settled) return undefined;
    let cancelled = false;
    renderCardGlint(settled).then(result => {
      if (cancelled) return;
      const target = still.current;
      target.width = result.canvas.width; target.height = result.canvas.height;
      target.getContext('2d').drawImage(result.canvas,0,0);
      setStillKey(settledKey);
    }).catch(error => {if (!cancelled) {console.error('[card-glint] Card render failed',error); setStillFailed(true);}});
    return () => {cancelled = true;};
  },[fallback,settled,settledKey]);

  const onPointerMove = event => {
    if (fallback) return;
    const box = event.currentTarget.getBoundingClientRect();
    live.pointer({x:(event.clientX-box.left)/box.width,y:(event.clientY-box.top)/box.height});
  };

  const shown = fallback ? Boolean(stillKey) : live.shown;
  const current = fallback ? stillKey === key : live.current && settledKey === key;
  const status = fallback ? (stillFailed ? 'error' : stillKey ? 'static' : 'pending') : shown ? 'live' : 'pending';
  const type = settled && api && !settled.faces?.length ? api.faceRects(settled).type : null;
  return <div ref={stage} className="card-glint-live" data-card-glint-live={status}
    data-card-glint-rendering={shown && !current ? 'true' : undefined}>
    <div className="card-glint-live__tilt" onPointerMove={onPointerMove} onPointerLeave={() => live.pointer(null)}>
      <canvas ref={liveCanvas} className="card-glint-live__canvas" hidden={fallback}
        role="img" aria-label={face ? `${face.name}: ${face.typeLine}. ${face.rulesText}` : undefined}/>
      <canvas ref={still} className="card-glint-live__canvas" hidden={!fallback}
        role="img" aria-label={face ? `${face.name}: ${face.typeLine}. ${face.rulesText}` : undefined}/>
      {shown && onBorderClick && BORDER_RAILS.map(([left,top,width,height],index) =>
        <button key={index} type="button" className="card-glint-live__target" tabIndex={index ? -1 : 0}
          style={{left:`${left}%`,top:`${top}%`,width:`${width}%`,height:`${height}%`}}
          aria-label={labels.border} title={labels.border} onClick={onBorderClick}/>)}
      {shown && type && onRarityClick && <button type="button" className="card-glint-live__target"
        style={{left:`${(type[0]+type[2])*100-10}%`,top:percent(type[1]),width:'9%',height:percent(type[3])}}
        aria-label={labels.rarity} title={labels.rarity} onClick={onRarityClick}/>}
    </div>
    {status === 'pending' && <div className="card-glint-live__message">{labels.pending}</div>}
    {status === 'error' && <div className="card-glint-live__message">{labels.error}</div>}
  </div>;
}
