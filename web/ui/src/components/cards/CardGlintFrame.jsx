import {useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {cardGlintFace} from '@/lib/card-glint/card.mjs';
import {renderCardGlint,peekCardGlintFrame} from '@/lib/card-glint/session.mjs';
import {peekScryfallPrintingMetadata,resolveScryfallPrintingMetadata} from '@/lib/scryfall';
import CardGlintInteractions from './CardGlintInteractions';
import {cardGlintRegions,cardGlintActionsCovered,cardGlintRulesFontSize} from '@/lib/card-glint/interactions.mjs';
import {cardPresentation} from '@/lib/card-glint/printing-choices.mjs';
import useCardGlintLive from './useCardGlintLive';

// In the inspector, a card with Build choices is drawn by the WebGL renderer
// over its static face, which still provides the interaction regions.
export default function CardGlintFrame({snapshot,imageUrl,faceIndex=0,fallback,rulesView,onActivate,highlighted,details,inspector=false}) {
  const canvas = useRef(null), [finished,setFinished] = useState(null), [metadata,setMetadata] = useState(null);
  const interactive=Boolean(rulesView);
  const [displayWidth,setDisplayWidth] = useState(null);
  useLayoutEffect(() => {
    if (!interactive) return;
    const host=canvas.current?.parentElement?.parentElement;
    if (!host) return;
    const measure=()=>setDisplayWidth(host.getBoundingClientRect().width);
    const observer=new ResizeObserver(measure);observer.observe(host);measure();
    return ()=>observer.disconnect();
  }, [interactive]);
  const card = useMemo(() => JSON.parse(snapshot), [snapshot]);
  const printing = metadata && metadata.imageUrl === imageUrl ? metadata.printing : peekScryfallPrintingMetadata(imageUrl);
  useEffect(() => {
    if (!imageUrl || peekScryfallPrintingMetadata(imageUrl) !== undefined) return;
    let cancelled = false;
    resolveScryfallPrintingMetadata(imageUrl).then(printing => {if (!cancelled) setMetadata({imageUrl,printing});})
      .catch(() => {if (!cancelled) setMetadata({imageUrl,printing:null});});
    return () => {cancelled = true;};
  }, [imageUrl]);
  const face = useMemo(() => cardGlintFace(card,{imageUrl,faceIndex,printing}), [card,imageUrl,faceIndex,printing]);
  useLayoutEffect(() => {
    // Paint from live card data immediately; printing metadata can refine
    // the decoration later without blocking text, art, or interactions.
    let cancelled = false;
    const paint = result => {
      if (cancelled) return;
      const target = canvas.current;
      target.width = result.canvas.width; target.height = result.canvas.height;
      target.getContext('2d').drawImage(result.canvas,0,0);
      setFinished({face,result});
    };
    const cached = peekCardGlintFrame(face);
    if (cached) {paint(cached);return () => {cancelled=true;};}
    renderCardGlint(face).then(paint).catch(error => {
      if (!cancelled) {console.error('[card-glint] Card render failed',face.name,error); setFinished({face,error});}
    });
    return () => {cancelled = true;};
  }, [face,imageUrl,printing]);
  // A warmed face is ready on its first render. Mounting the text fallback
  // before the layout effect paints this cache hit needlessly runs its fitter.
  const result = finished?.face === face ? finished.result : peekCardGlintFrame(face);
  const presentation = inspector ? cardPresentation(card) : null;
  const [liveCanvas,live] = useCardGlintLive(face,presentation,{enabled:Boolean(presentation)});
  const foil = live.mode === 'live' && live.current;
  const onPointerMove = event => {
    const box = event.currentTarget.getBoundingClientRect();
    live.pointer({x:(event.clientX-box.left)/box.width,y:(event.clientY-box.top)/box.height});
  };
  const regions = useMemo(() => cardGlintRegions(result,rulesView), [result,rulesView]);
  const paintedFontSize=useMemo(()=>rulesView?cardGlintRulesFontSize(result):null,[result,rulesView]);
  const readable=!rulesView || paintedFontSize==null || displayWidth==null || paintedFontSize*displayWidth/488>=14;
  const ready = Boolean(result && readable && cardGlintActionsCovered(regions,rulesView));
  return <>
    {!ready && fallback}
    <div className="card-glint-surface" style={{display:ready?'block':'none'}}
      onPointerMove={presentation ? onPointerMove : undefined} onPointerLeave={presentation ? () => live.pointer(null) : undefined}>
      <canvas ref={canvas} data-card-glint-frame={ready?'ready':result?'unavailable':finished?.face===face&&finished.error?'error':'pending'}
        data-card-glint-error={finished?.face===face?finished.error?.message:undefined}
        aria-label={`${face.name}: ${face.typeLine}. ${face.rulesText}`} role="img"/>
      {presentation && <canvas ref={liveCanvas} className="card-glint-surface__live" aria-hidden="true"
        data-card-glint-finish={foil?presentation.finish:undefined} style={{visibility:foil?'visible':'hidden'}}/>}
      {ready && rulesView && <CardGlintInteractions result={result} regions={regions} rulesView={rulesView}
        onActivate={onActivate} highlighted={highlighted} name={face.name}/>}
      {details && <span className="mse-card-live-details">{details}</span>}
    </div>
  </>;
}
