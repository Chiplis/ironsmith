import {useEffect,useState} from 'react';
import {createRoot} from 'react-dom/client';
import CardPrintingStage from '../src/components/layout/CardPrintingStage';
import CardFramePresentation from '../src/components/cards/CardFramePresentation';
import {I18nProvider} from '../src/i18n/I18nContext';
import {DEFAULT_CARD_PRESENTATION,setCardPresentation} from '../src/lib/card-glint/printing-choices.mjs';
import '../src/index.css';
const face={name:'Forge Test Adept',manaCost:'{2}{R}',colorIndicator:[],supertypes:[],cardTypes:['Creature'],subtypes:['Human','Wizard'],
  oracleText:'Haste\nWhen this creature enters, draw a card.',artUrl:'',power:'3',toughness:'2',loyalty:'',defense:''};
// The table copy added from Build has its own printing; another copy does not.
const tableCard=stable=>({id:stable,stable_id:stable,name:'Forge Test Adept',mana_cost:'{2}{R}',type_line:'Creature — Human Wizard',
  oracle_text:'Haste',power:'3',toughness:'2'});
setCardPresentation(501,['Forge Test Adept'],{finish:'prismatic',printEffects:true,style:'4ed'});
const view=new URLSearchParams(location.search).get('view');
function Fixture(){
  const [draft,setDraft]=useState({layout:'single',hasFuse:false,faces:[face],presentation:{...DEFAULT_CARD_PRESENTATION}});
  useEffect(()=>{
    Object.assign(window,{setBuildFace:patch=>setDraft(current=>({...current,faces:[{...current.faces[0],...patch}]}))});
  },[]);
  if(view==='table')return <I18nProvider><div style={{display:'flex',gap:20,margin:20}}>
    {[501,502].map(stable=><div key={stable} data-table-card={stable} style={{width:366,height:510}}>
      <CardFramePresentation card={tableCard(stable)} inspector/></div>)}
  </div></I18nProvider>;
  return <I18nProvider><div className="card-forge-sheet" style={{width:440,height:1040,margin:20,display:'grid'}}>
    <CardPrintingStage draft={draft} faceIndex={0} onPresentationChange={presentation=>setDraft(current=>({...current,presentation}))}/>
  </div></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
