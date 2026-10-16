import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {I18nProvider} from '../src/i18n/I18nContext';
import CardGlintZonePreparation from '../src/components/cards/CardGlintZonePreparation';
import CardFramePresentation from '../src/components/cards/CardFramePresentation';
import {cardGlintFace} from '../src/lib/card-glint/card.mjs';
import {hasCardGlintFrame,renderCardGlint} from '../src/lib/card-glint/session.mjs';
import '../src/index.css';
const source=document.createElement('canvas');source.width=40;source.height=40;
const context=source.getContext('2d');context.fillStyle='#2030a0';context.fillRect(0,0,40,40);
const art=source.toDataURL();
const hand={id:1,name:'Prepared hand',type_line:'Creature — Wizard',oracle_text:'Draw a card.',colors:['U'],power:'1',toughness:'1'};
const field={...hand,id:2,name:'Prepared battlefield'};
function Fixture(){
  const [shown,setShown]=useState(false),[power,setPower]=useState('1');
  const current={...hand,power};
  window.glintBackgroundPending=()=>[current,field].every(card=>hasCardGlintFrame(cardGlintFace(card,{imageUrl:art,printing:null})));
  window.finishGlintBackground=()=>Promise.all([current,field].map(card=>renderCardGlint(cardGlintFace(card,{imageUrl:art,printing:null}),{background:true})));
  window.showGlintBackground=()=>setShown(true);
  window.changeGlintBackground=()=>setPower('5');
  return <I18nProvider><GameContext.Provider value={{state:{perspective:0,players:[{id:0,hand_cards:[current],battlefield:[field]}]}}}>
    <div hidden data-object-id="1" data-card-image-url={art}/><div hidden data-object-id="2" data-card-image-url={art}/>
    <CardGlintZonePreparation/>
    {shown && <div style={{width:366,height:510}}><CardFramePresentation card={current} imageUrl={art}/></div>}
  </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
