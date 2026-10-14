import {useLayoutEffect,useMemo} from 'react';
import MseCardFrame from './MseCardFrame';
import SimpleCardFrame from './SimpleCardFrame';
import CardFrameRules from './CardFrameRules';
import {cardFrameSnapshot} from '@/lib/card-frame-snapshot.mjs';

// The inspector's rules are interactive within either rendered card surface.
export default function CardFramePresentation({card,imageUrl,faceIndex=0,rulesView,onActivate,highlighted,
  counters,badges,onReadyChange}) {
  const snapshot=cardFrameSnapshot(card);
  const printable=useMemo(()=>JSON.parse(snapshot),[snapshot]);
  useLayoutEffect(()=>{onReadyChange?.(true);},[onReadyChange]);
  return <div className="card-frame-presentation" data-frame-presentation={rulesView?'inspector':'miniature'} data-render-ready="true">
    <div className="card-frame-presentation__visual">
    <MseCardFrame snapshot={snapshot} imageUrl={imageUrl} style={card.mse_style} faceIndex={faceIndex}
      rulesView={rulesView} onActivate={onActivate} highlighted={highlighted}
      details={[counters,...(badges||[])].filter(Boolean).join(' · ')}
      fallback={<SimpleCardFrame card={printable} imageUrl={imageUrl} faceIndex={faceIndex}
        details={[counters,...(badges||[])].filter(Boolean).join(' · ')}
        rulesContent={rulesView&&<CardFrameRules name={card.name} rulesView={rulesView} onActivate={onActivate} highlighted={highlighted}/>}/>}/>
    </div>
  </div>;
}
