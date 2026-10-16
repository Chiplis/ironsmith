import {useState,useLayoutEffect} from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {I18nProvider} from '../src/i18n/I18nContext';
import CardGlintZonePreparation from '../src/components/cards/CardGlintZonePreparation';
import CardFramePresentation from '../src/components/cards/CardFramePresentation';
import {cardGlintFace} from '../src/lib/card-glint/card.mjs';
import {hasCardGlintFrame,peekCardGlintFrame} from '../src/lib/card-glint/session.mjs';
import '../src/index.css';
const source=document.createElement('canvas');source.width=40;source.height=40;
const context=source.getContext('2d');context.fillStyle='#2030a0';context.fillRect(0,0,40,40);
const art=source.toDataURL();
const hand={id:1,name:'Prepared hand',type_line:'Creature — Wizard',oracle_text:'Draw a card.',colors:['U'],flavor_text:'A prepared wizard.',power:'1',toughness:'1'};
const field={...hand,id:2,name:'Prepared battlefield'};
const game={objectDetails:async id=>({...game.cards.find(card=>card.id===Number(id)),compiled_text:['Draw a card.']})};
function Fixture(){
  const [shown,setShown]=useState(false),[power,setPower]=useState('1');
  const current={...hand,power};
  window.glintBackgroundPending=()=>[current,field].every(card=>hasCardGlintFrame(cardGlintFace(card,{imageUrl:art,printing:null})));
  game.cards=[current,field];
  window.finishGlintBackground=async()=>{
    const start=performance.now();
    while(![current,field].every(card=>peekCardGlintFrame(cardGlintFace(card,{imageUrl:art,printing:null})))){
      if(performance.now()-start>30000)throw Error('background did not complete the inspector faces');
      await new Promise(resolve=>setTimeout(resolve,20));
    }
  };
  useLayoutEffect(()=>{if(shown)requestAnimationFrame(()=>{window.glintReadyAtFirstPaint=Boolean(document.querySelector('[data-card-glint-frame="ready"]'))&&!document.querySelector('.simple-card-frame');});},[shown]);
  const lightweight=card=>({id:card.id,name:card.name,card_types:['creature'],power_toughness:card.power+'/1'});
  window.showGlintBackground=()=>setShown(true);
  window.changeGlintBackground=()=>setPower('5');
  return <I18nProvider><GameContext.Provider value={{game,state:{perspective:0,players:[{id:0,hand_cards:[lightweight(current)],battlefield:[lightweight(field)]}]}}}>
    <div hidden data-object-id="1" data-card-image-url={art}/><div hidden data-object-id="2" data-card-image-url={art}/>
    <CardGlintZonePreparation/>
    {shown && <div style={{width:366,height:510}}><CardFramePresentation card={current} imageUrl={art}/></div>}
  </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
