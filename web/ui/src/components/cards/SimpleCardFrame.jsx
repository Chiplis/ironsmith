import {useLayoutEffect,useRef,useState} from 'react';
import {ManaCostIcons,SymbolText} from '@/lib/mana-symbols';
import {cardFrameTone} from '@/lib/card-frame-tone';
import {cardArtCropUrl} from '@/lib/card-image-variants';
import {cardFrameRules} from '@/lib/card-frame-snapshot.mjs';
import useUiText from '@/i18n/useUiText';
import './card-frame-presentation.css';

// One self-contained fallback. Artwork may load or fail, but the frame and
// live text never depend on a scan, generated mask, or stock-frame image.
export default function SimpleCardFrame({card,imageUrl,faceIndex=0,rulesContent,details}) {
  const ui=useUiText(),rules=useRef(null),[failedImage,setFailedImage]=useState(null);
  const faces=card.card_faces?.length?card.card_faces:[card];
  const multipart=['split','adventure'].includes(card.layout);
  const face=faces[faceIndex]??faces[0];
  const shown=multipart?faces:[face];
  const type=face.type_line_display??face.type_line??card.type_line_display??card.type_line;
  const tone=cardFrameTone({cards:[face,card],manaCost:face.mana_cost??card.mana_cost,typeLine:type});
  const art=cardArtCropUrl(imageUrl);
  const textKey=shown.map(value=>cardFrameRules(value)+'\n'+(value.flavor_text??'')).join('\n');
  useLayoutEffect(()=>{
    const box=rules.current;
    let frame=0;
    const fit=()=>{
      if(!box.clientHeight||!box.clientWidth)return;
      box.style.removeProperty('font-size');
      const preferred=parseFloat(getComputedStyle(box).fontSize);
      let low=Math.min(7,preferred),high=preferred;
      for(let i=0;i<10;i++){
        const size=(low+high)/2;box.style.fontSize=`${size}px`;
        if(box.scrollHeight>box.clientHeight+1||box.scrollWidth>box.clientWidth+1)high=size;else low=size;
      }
      box.style.fontSize=`${low}px`;
    };
    const schedule=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(fit);};
    const observer=new ResizeObserver(schedule);observer.observe(box);
    fit();document.fonts.addEventListener('loadingdone',schedule);
    return()=>{observer.disconnect();cancelAnimationFrame(frame);document.fonts.removeEventListener('loadingdone',schedule);};
  },[textKey]);
  const stats=face.power!=null&&face.toughness!=null?`${face.power}/${face.toughness}`
    :face.power_toughness??face.defense??face.loyalty;
  return <article className="simple-card-frame" data-frame-mode="simple" data-card-frame-tone={tone.tone}
    style={tone.style} aria-label={face.name||card.name||ui('Card details')}>
    <header className="simple-card-frame__heading"><strong>{face.name||card.name}</strong>
      <ManaCostIcons cost={face.mana_cost??card.mana_cost} size="1em" /></header>
    <div className="simple-card-frame__art">
      {art&&failedImage!==art&&<img src={art} alt="" decoding="async" referrerPolicy="no-referrer" onError={()=>setFailedImage(art)}/>}
    </div>
    <div className="simple-card-frame__type">{type||ui('Card')}</div>
    <div className="simple-card-frame__rules" ref={rules}>
      {rulesContent?<>{rulesContent}{face.flavor_text&&<p className="simple-card-frame__flavor">{face.flavor_text}</p>}</>:shown.map((part,index)=><section key={index}>
        {index>0&&<><header className="simple-card-frame__heading"><strong>{part.name}</strong><ManaCostIcons cost={part.mana_cost} size="1em"/></header>
          <div className="simple-card-frame__type">{part.type_line_display??part.type_line}</div></>}
        {cardFrameRules(part).split('\n').filter(Boolean).map((line,i)=><p key={i}><SymbolText text={line}/></p>)}
        {part.flavor_text&&<p className="simple-card-frame__flavor">{part.flavor_text}</p>}
      </section>)}
    </div>
    <footer className="simple-card-frame__footer">{details&&<span className="card-frame-live-details">{details}</span>}{stats!=null&&<strong>{stats}</strong>}</footer>
  </article>;
}
