import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import CardFramePresentation from '../src/components/cards/CardFramePresentation';
import {I18nProvider} from '../src/i18n/I18nContext';
import {prepareCardGlintFace} from '../src/lib/card-glint/preparation.mjs';
import {renderCardGlint} from '../src/lib/card-glint/session.mjs';
import '../src/index.css';
const initial={name:'Glint Test Adept',mana_cost:'{2}{U}',colors:['U'],type_line:'Creature — Human Wizard',
  oracle_text:'{T}: Draw a card.',power:'2',toughness:'3',set:'m21',rarity:'common'};
function Fixture(){
  const [rules,setRules]=useState(null);
  window.setGlintRules=setRules;
  const [card,setCard]=useState(initial), [art,setArt]=useState(undefined);
  const [shown,setShown]=useState(!new URLSearchParams(location.search).has('warm'));
  window.showGlintFrame=()=>setShown(true);
  window.warmGlintFrame=async()=>{
    const face=await prepareCardGlintFace(initial,{resolveImage:async()=>null});
    window.warmedGlintResult=await renderCardGlint(face,{background:true});
  };
  window.setGlintArt=setArt;
  window.changeGlintCard = setCard;
  return <I18nProvider><div style={{width:366,height:510,margin:20}}>
    {shown && <CardFramePresentation card={card} imageUrl={art} rulesView={rules ?? {lines:[card.oracle_text],actions:new Map([[0,[{id:'draw'}]]])}}
      onActivate={action=>{window.glintActivated=action.id;}}/>}
  </div></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
