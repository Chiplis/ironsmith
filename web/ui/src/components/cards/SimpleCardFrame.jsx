import {useLayoutEffect,useRef} from 'react';
import {ManaCostIcons,SymbolText} from '@/lib/mana-symbols';
import {cardFrameTone} from '@/lib/card-frame-tone';
import {cardArtCropUrl} from '@/lib/card-image-variants';
import {cardFrameRules} from '@/lib/card-frame-snapshot.mjs';
import LoadingCardArt from './LoadingCardArt';
import {cardArtColors} from '@/lib/card-art-colors';
import {fallbackCardFrameColors} from '@/lib/default-card-frames';
import stockFrames from '@/lib/default-card-frames.generated.json';
import useUiText from '@/i18n/useUiText';
import {rulesSourceFrame} from '@/lib/rules-source-frame';
import './card-frame-presentation.css';

// Shipped stock frames need no renderer; rules-source cards keep their CSS frame.
export default function SimpleCardFrame({card,imageUrl,faceIndex=0,rulesContent,details,baked=false}) {
  const ui=useUiText(),rules=useRef(null);
  const interactive=Boolean(rulesContent);
  const faces=card.card_faces?.length?card.card_faces:[card];
  const multipart=['split','aftermath','adventure','prepare','room','flip'].includes(card.layout);
  const face=faces[faceIndex]??faces[0];
  const shown=multipart?faces:[face];
  const type=face.type_line_display??face.type_line??card.type_line_display??card.type_line;
  const tone=cardFrameTone({cards:[face,card],manaCost:face.mana_cost??card.mana_cost,typeLine:type});
  const art=cardArtCropUrl(imageUrl);
  const sourceFrame=rulesSourceFrame(card);
  const colors=cardArtColors(face);
  const frameColors=/\bLand\b/i.test(type||'')
    ? fallbackCardFrameColors({cards:[face,card],typeLine:type}) : colors;
  const frameKey=frameColors.join('')||(/\bLand\b/i.test(type||'')?'land':/\bArtifact\b/i.test(type||'')?'C':'colorless');
  const stock=baked&&!sourceFrame?stockFrames[frameKey]:null;
  const box=name=>stock?{
    left:`${100*stock.style[`--printed-${name}-x`]/stock.style['--printed-scan-width']}%`,
    top:`${100*stock.style[`--printed-${name}-y`]/stock.style['--printed-scan-height']}%`,
    width:`${100*stock.style[`--printed-${name}-width`]/stock.style['--printed-scan-width']}%`,
    height:`${100*stock.style[`--printed-${name}-height`]/stock.style['--printed-scan-height']}%`,
  }:undefined;
  const textKey=shown.map(value=>cardFrameRules(value)+'\n'+(value.flavor_text??'')).join('\n');
  useLayoutEffect(()=>{
    const box=rules.current;
    let frame=0,lastWidth=-1,lastHeight=-1;
    const fit=()=>{
      const width=box.clientWidth,height=box.clientHeight;
      if(!width||!height || (width===lastWidth && height===lastHeight))return;
      lastWidth=width;lastHeight=height;
      box.style.removeProperty('font-size');
      const preferred=parseFloat(getComputedStyle(box).fontSize);
      // An interactive rules area scrolls instead of shrinking its controls.
      if(interactive){box.style.fontSize=`${Math.max(14,preferred)}px`;return;}
      // Most temporary frames already fit (and some have no rules at all).
      // Do not run ten write/layout cycles before replacing them with canvas.
      const fits=()=>box.scrollHeight<=box.clientHeight+1 && box.scrollWidth<=box.clientWidth+1;
      if(fits())return;
      let low=Math.min(7,preferred),high=preferred;
      for(let i=0;i<10;i++){
        const size=(low+high)/2;box.style.fontSize=`${size}px`;
        if(fits())low=size;else high=size;
      }
      box.style.fontSize=`${low}px`;
    };
    const schedule=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(fit);};
    const fontsChanged=()=>{lastWidth=-1;schedule();};
    const observer=new ResizeObserver(schedule);observer.observe(box);
    fit();document.fonts.addEventListener('loadingdone',fontsChanged);
    return()=>{observer.disconnect();cancelAnimationFrame(frame);document.fonts.removeEventListener('loadingdone',fontsChanged);};
  },[textKey,interactive]);
  const stats=face.power!=null&&face.toughness!=null?`${face.power}/${face.toughness}`
    :face.power_toughness??face.defense??face.loyalty;
  return <article className="simple-card-frame" data-frame-mode={stock?"baked":"simple"} data-card-frame-tone={tone.tone}
    data-interactive-rules={rulesContent ? "true" : undefined}
    data-rules-source={sourceFrame ? card.source_kind : undefined}
    style={{...tone.style,...(sourceFrame ? {'--rules-source-color':sourceFrame.color} : {})}} aria-label={face.name||card.name||ui('Card details')}>
    {stock&&<img className="simple-card-frame__stock" src={`${import.meta.env.BASE_URL}card-frames/default/${frameKey}.webp`} alt=""/>}
    <header className="simple-card-frame__heading" style={box('title')}><strong>{face.name||card.name}</strong>
      <ManaCostIcons cost={face.display_mana_cost??card.display_mana_cost??face.mana_cost??card.mana_cost} size="1em" /></header>
    <div className="simple-card-frame__art" style={box('art')}>
      {sourceFrame&&<div className="rules-source-emblem" aria-hidden="true"><span>{sourceFrame.symbol}</span><strong>{sourceFrame.label}</strong></div>}
      {(art||!sourceFrame)&&<LoadingCardArt src={art} pending={!imageUrl} colors={colors} variant="hand" alt="" decoding="async" referrerPolicy="no-referrer"/>}
    </div>
    <div className="simple-card-frame__type" style={box('type')}>{type||ui('Card')}</div>
    <div className="simple-card-frame__rules" ref={rules} style={box('rules')}>
      {rulesContent?<>{rulesContent}{face.flavor_text&&<p className="simple-card-frame__flavor">{face.flavor_text}</p>}</>:shown.map((part,index)=><section key={index}>
        {index>0&&<><header className="simple-card-frame__heading"><strong>{part.name}</strong><ManaCostIcons cost={part.display_mana_cost??part.mana_cost} size="1em"/></header>
          <div className="simple-card-frame__type">{part.type_line_display??part.type_line}</div></>}
        {cardFrameRules(part).split('\n').filter(Boolean).map((line,i)=><p key={i}><SymbolText text={line}/></p>)}
        {part.flavor_text&&<p className="simple-card-frame__flavor">{part.flavor_text}</p>}
      </section>)}
    </div>
    <footer className="simple-card-frame__footer">{details&&<span className="card-frame-live-details">{details}</span>}{stats!=null&&<strong>{stats}</strong>}</footer>
  </article>;
}
