import {useEffect,useLayoutEffect,useMemo,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {HoverProvider} from '../src/context/HoverContext';
import {DragProvider} from '../src/context/DragContext';
import {CombatArrowProvider} from '../src/context/CombatArrowContext';
import {I18nProvider} from '../src/i18n/I18nContext';
import {TooltipProvider} from '../src/components/ui/tooltip';
import {useHandManaCosts} from '../src/hooks/useHandManaCosts';
import GameCard from '../src/components/cards/GameCard';
import '../src/index.css';
const card={id:7,stable_id:7,name:'Mana Cost Adept',mana_cost:'{2}{U}',type_line:'Creature — Wizard',
  oracle_text:'Flying',power:'2',toughness:'2',card_types:['creature'],colors:['U']};
const art=document.createElement('canvas');art.width=100;art.height=100;
art.getContext('2d').fillStyle='#304c76';art.getContext('2d').fillRect(0,0,100,100);
export function Fixture(){
  const [state,setState]=useState(()=>({snapshot_id:1,perspective:0,active_player:0,players:[{id:0,name:'Alice',hand_cards:[card],can_view_hand:true,hand_mana_costs:[]}],decision:null}));
  const stateRef=useRef(state); useLayoutEffect(()=>{stateRef.current=state;},[state]);
  const game=useMemo(()=>({previewHandManaCosts:()=>new Promise(resolve=>{
    window.resolveHandPrice=mana_cost=>resolve({snapshot_id:1,perspective:0,prices:[{id:card.id,name:card.name,mana_cost}]});
  })}),[]);
  useHandManaCosts({game,state,stateRef,setState});
  useEffect(()=>{
    window.setHandManaCost=cost=>setState(current=>({...current,players:current.players.map(player=>({...player,hand_mana_costs:cost==null?[]:[{id:card.id,name:card.name,mana_cost:cost}]}))}));
    window.printedHandCard=card;
  },[]);
  const cost=state.players[0].hand_mana_costs[0]?.mana_cost;
  return <I18nProvider><GameContext.Provider value={{state,playerAccentOverrides:{}}}>
    <HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
      <main style={{padding:24,width:430,height:650,background:'#17131e',color:'#e4dbea'}}>
        <p>Printed cost: {'{2}{U}'}</p><p>Current casting cost: {cost??'{2}{U}'}</p>
        <GameCard card={card} variant="hand" sourceImageUrl={art.toDataURL()} onClick={()=>{}}
          style={{width:300,maxWidth:300,height:420,position:'relative',flex:'none'}} />
      </main>
    </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider>
  </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
