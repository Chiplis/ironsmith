import {useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {cardGlintFace} from '@/lib/card-glint/card.mjs';
import {renderCardGlint,peekCardGlintFrame} from '@/lib/card-glint/session.mjs';
import {peekScryfallPrintingMetadata,resolveScryfallPrintingMetadata} from '@/lib/scryfall';
import CardGlintInteractions from './CardGlintInteractions';
import {cardGlintRegions,cardGlintActionsCovered} from '@/lib/card-glint/interactions.mjs';

export default function CardGlintFrame({snapshot,imageUrl,faceIndex=0,fallback,rulesView,onActivate,highlighted,details}) {
  const canvas = useRef(null), [finished,setFinished] = useState(null), [metadata,setMetadata] = useState(null);
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
    // Avoid painting a provisional frame before the selected printing arrives.
    if (imageUrl && printing === undefined) return;
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
  const result = finished?.face === face ? finished.result : null;
  const regions = useMemo(() => cardGlintRegions(result,rulesView), [result,rulesView]);
  const ready = Boolean(result && cardGlintActionsCovered(regions,rulesView));
  return <>
    {!ready && fallback}
    <div className="card-glint-surface" style={{display:ready?'block':'none'}}>
      <canvas ref={canvas} data-card-glint-frame={ready?'ready':result?'unavailable':finished?.face===face&&finished.error?'error':'pending'}
        data-card-glint-error={finished?.face===face?finished.error?.message:undefined}
        aria-label={`${face.name}: ${face.typeLine}. ${face.rulesText}`} role="img"/>
      {ready && rulesView && <CardGlintInteractions result={result} regions={regions} rulesView={rulesView}
        onActivate={onActivate} highlighted={highlighted} name={face.name}/>}
      {details && <span className="mse-card-live-details">{details}</span>}
    </div>
  </>;
}
