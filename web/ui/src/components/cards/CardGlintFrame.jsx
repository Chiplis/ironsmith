import {useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {cardGlintFace} from '@/lib/card-glint/card.mjs';
import {renderCardGlint,peekCardGlintFrame,renderCardGlintWithoutRules} from '@/lib/card-glint/session.mjs';
import {peekScryfallPrintingMetadata,resolveScryfallPrintingMetadata} from '@/lib/scryfall';
import CardGlintInteractions,{CardGlintRuleHelpers} from './CardGlintInteractions';
import CardGlintScrollableRules from './CardGlintScrollableRules';
import {cardGlintRegions,cardGlintActionsCovered,cardGlintRulesFontSize} from '@/lib/card-glint/interactions.mjs';
import {cardPresentation} from '@/lib/card-glint/printing-choices.mjs';
import useCardGlintLive from './useCardGlintLive';
import useBattlefieldAppearance from '@/hooks/useBattlefieldAppearance';

// Draws `source` into `target` at the target's layout width in device pixels,
// never above the source's own resolution. Large reductions halve first so
// every step stays within what smoothed drawImage samples cleanly.
function paintDisplayCanvas(target, source) {
  if (!target || !source?.width || !source.height) return;
  const layoutWidth = target.clientWidth;
  const ratio = typeof window === 'undefined' ? 1 : window.devicePixelRatio || 1;
  const width = layoutWidth > 0 ? Math.min(source.width, Math.ceil(layoutWidth * ratio)) : source.width;
  const height = Math.max(1, Math.round(width * source.height / source.width));
  let step = source;
  while (step.width > width * 2) {
    const half = document.createElement('canvas');
    half.width = Math.ceil(step.width / 2); half.height = Math.ceil(step.height / 2);
    const context = half.getContext('2d');
    context.imageSmoothingQuality = 'high';
    context.drawImage(step,0,0,half.width,half.height);
    step = half;
  }
  if (target.width !== width || target.height !== height) {target.width = width; target.height = height;}
  const context = target.getContext('2d');
  context.clearRect(0,0,width,height);
  context.imageSmoothingEnabled = true;
  context.imageSmoothingQuality = 'high';
  context.drawImage(step,0,0,width,height);
}

// In the inspector, a card with Build choices is drawn by the WebGL renderer
// over its static face, which still provides the interaction regions.
export default function CardGlintFrame({snapshot,imageUrl,faceIndex=0,fallback,rulesView,onActivate,highlighted,details,inspector=false,keywordHelpers=false}) {
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
    // A stalled lookup must not hold the fallback forever: paint undecorated.
    const grace = setTimeout(() => setMetadata(current => current?.imageUrl === imageUrl ? current : {imageUrl,printing:null}), 2000);
    resolveScryfallPrintingMetadata(imageUrl).then(printing => {if (!cancelled) setMetadata({imageUrl,printing});})
      .catch(() => {if (!cancelled) setMetadata({imageUrl,printing:null});})
      .finally(() => clearTimeout(grace));
    return () => {cancelled = true; clearTimeout(grace);};
  }, [imageUrl]);
  const face = useMemo(() => cardGlintFace(card,{imageUrl,faceIndex,printing}), [card,imageUrl,faceIndex,printing]);
  // `undefined` means the printing lookup is still in flight (null: settled,
  // none). Painting before it settles shows an undecorated frame that is
  // replaced moments later, so the fallback stays up until the real one.
  const printingPending = printing === undefined;
  useLayoutEffect(() => {
    if (printingPending) return undefined;
    let cancelled = false;
    const paint = result => {
      if (cancelled) return;
      paintDisplayCanvas(canvas.current,result.canvas);
      setFinished({face,result});
    };
    const cached = peekCardGlintFrame(face);
    if (cached) {paint(cached);return () => {cancelled=true;};}
    renderCardGlint(face).catch(error=>{
      if (!cancelled && interactive && error.message==='Magic rules text does not fit the available area.')
        return renderCardGlintWithoutRules(face);
      throw error;
    }).then(paint).catch(error => {
      if (!cancelled) {console.error('[card-glint] Card render failed',face.name,error); setFinished({face,error});}
    });
    return () => {cancelled = true;};
  }, [face,imageUrl,printing,printingPending,interactive]);
  // A warmed face is ready on its first render. Mounting the text fallback
  // before the layout effect paints this cache hit needlessly runs its fitter.
  const result = printingPending ? undefined : finished?.face === face ? finished.result : peekCardGlintFrame(face);
  // The painted texture is the inspector tier (732px wide). Showing it in a
  // small slot leaves the minification to the compositor, whose bilinear
  // sampling without mipmaps skips texels past 2x: on a 1x display a hand
  // card shrinks it ~4x and the frame text turns soft and jagged. Repaint at
  // the canvas's untransformed size in device pixels instead, which is also
  // the size a hovered hand card grows to.
  const source = result?.canvas;
  useLayoutEffect(() => {
    const target = canvas.current;
    if (!target || !source) return;
    const repaint = () => paintDisplayCanvas(target,source);
    const observer = new ResizeObserver(repaint);
    observer.observe(target);
    let media;
    const watchRatio = () => {
      media?.removeEventListener('change',onRatio);
      media = window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`);
      media.addEventListener('change',onRatio);
    };
    function onRatio() {watchRatio(); repaint();}
    watchRatio();
    return () => {observer.disconnect(); media?.removeEventListener('change',onRatio);};
  }, [source]);
  const presentation = inspector ? cardPresentation(card) : null;
  const [{cardFinishEffects}] = useBattlefieldAppearance();
  const [liveCanvas,live] = useCardGlintLive(face,presentation,{enabled:Boolean(presentation)&&cardFinishEffects});
  const foil = live.mode === 'live' && live.current;
  const onPointerMove = event => {
    const box = event.currentTarget.getBoundingClientRect();
    live.pointer({x:(event.clientX-box.left)/box.width,y:(event.clientY-box.top)/box.height});
  };
  const regions = useMemo(() => result?.rulesOmitted?[]:cardGlintRegions(result,rulesView), [result,rulesView]);
  // Help targets resolve against the painted rules text itself.
  const helperView = useMemo(() => keywordHelpers && !rulesView && !result?.rulesOmitted
    ? {lines:String(result?.face.rulesText||'').split('\n').filter(line=>line.trim()),actions:new Map()} : null,
  [keywordHelpers,rulesView,result]);
  const paintedFontSize=useMemo(()=>rulesView&&!result?.rulesOmitted?cardGlintRulesFontSize(result):null,[result,rulesView]);
  const readable=!rulesView || paintedFontSize==null || displayWidth==null || paintedFontSize*displayWidth/488>=12;
  const covered=cardGlintActionsCovered(regions,rulesView);
  const kind=result?.api.faceFrame(result.face);
  const scrollable=Boolean(result&&rulesView&&(result.rulesOmitted||!readable||!covered)&&!result.face.faces?.length
    && ['normal','planeswalker','planeswalker-large','saga'].includes(kind));
  const ready = Boolean(result && (scrollable || !result.rulesOmitted && readable && covered));
  return <>
    {!ready && fallback}
    <div className="card-glint-surface" style={{display:ready?'block':'none'}}
      onPointerMove={presentation ? onPointerMove : undefined} onPointerLeave={presentation ? () => live.pointer(null) : undefined}>
      <canvas ref={canvas} data-card-glint-frame={ready?'ready':result?'unavailable':finished?.face===face&&finished.error?'error':'pending'}
        data-card-glint-error={finished?.face===face?finished.error?.message:undefined}
        aria-label={`${face.name}: ${face.typeLine}. ${face.rulesText}`} role="img"/>
      {presentation && <canvas ref={liveCanvas} className="card-glint-surface__live" aria-hidden="true"
        data-card-glint-finish={foil?presentation.finish:undefined} style={{visibility:foil&&!scrollable?'visible':'hidden'}}/>}
      {ready && scrollable && <CardGlintScrollableRules result={result} rulesView={rulesView}
        onActivate={onActivate} highlighted={highlighted} name={face.name}/>}
      {ready && rulesView && !scrollable && <CardGlintInteractions result={result} regions={regions} rulesView={rulesView}
        onActivate={onActivate} highlighted={highlighted} name={face.name}/>}
      {ready && helperView && <div className="mse-card-interactions"><CardGlintRuleHelpers result={result} rulesView={helperView} name={face.name}/></div>}
      {details && <span className="mse-card-live-details">{details}</span>}
    </div>
  </>;
}
