import {cardFlavorSource} from '@/lib/card-flavor-source';
import {useLayoutEffect,useMemo} from 'react';
import CardGlintFrame from './CardGlintFrame';
import SimpleCardFrame from './SimpleCardFrame';
import CardFrameRules from './CardFrameRules';
import {cardFrameSnapshot} from '@/lib/card-frame-snapshot.mjs';
import {rulesSourceCard,rulesSourceFrame} from '@/lib/rules-source-frame';

// The inspector's rules are interactive within either rendered card surface.
export default function CardFramePresentation({card,imageUrl,faceIndex=0,rulesView,onActivate,highlighted,
  counters,badges,onReadyChange}) {
  const snapshot=cardFrameSnapshot(card);
  const printable=useMemo(()=>JSON.parse(snapshot),[snapshot]);
  const source=rulesView ? cardFlavorSource(printable.card_faces?.[faceIndex]?.flavor_text ?? printable.flavor_text) : null;
  const fallbackCard=source ? {...printable,flavor_text:'',
    ...(printable.card_faces ? {card_faces:printable.card_faces.map((face,index)=>index===faceIndex?{...face,flavor_text:''}:face)} : {})} : printable;
  useLayoutEffect(()=>{onReadyChange?.(true);},[onReadyChange]);
  return <div className="card-frame-presentation" data-frame-presentation={rulesView?'inspector':'miniature'} data-render-ready="true">
    <div className="card-frame-presentation__visual">
    {rulesSourceFrame(printable) ? <SimpleCardFrame card={rulesSourceCard(printable)}
      rulesContent={rulesView&&<CardFrameRules name={card.name} rulesView={rulesView} onActivate={onActivate} highlighted={highlighted}/>}/>
    : <CardGlintFrame snapshot={snapshot} imageUrl={imageUrl} faceIndex={faceIndex}
      rulesView={rulesView} onActivate={onActivate} highlighted={highlighted}
      details={[counters,...(badges||[])].filter(Boolean).join(' · ')}
      fallback={<SimpleCardFrame card={fallbackCard} imageUrl={imageUrl} faceIndex={faceIndex}
        details={[counters,...(badges||[])].filter(Boolean).join(' · ')}
        rulesContent={rulesView&&<CardFrameRules name={card.name} rulesView={rulesView} onActivate={onActivate} highlighted={highlighted}/>}/>}/>}
    </div>
    {source && <a className="card-frame-source-link" href={source.href} target="_blank" rel="noopener noreferrer"
      title={source.href} aria-label={`Open original post on ${source.host}`}
      onPointerDown={event=>event.stopPropagation()} onClick={event=>event.stopPropagation()}>
      Original post · {source.host} <span aria-hidden="true">↗</span>
    </a>}
  </div>;
}
