import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {HoverProvider} from '../src/context/HoverContext';
import {DragProvider} from '../src/context/DragContext';
import {I18nProvider} from '../src/i18n/I18nContext';
import GameCard from '../src/components/cards/GameCard';
import '../src/index.css';
import {createForgeMeasurer} from '../src/components/board/forge/measure-forge';
window.measureCardShapes = () => createForgeMeasurer(document.querySelector('#shape-fixture'))();
function Fixture(){
 const [sick,setSick]=useState(true);
 const card={id:1,name:'Counter creature',lane:'creatures',power_toughness:'4/4',summoning_sick:sick,counters:[{kind:'+1/+1',amount:2},{kind:'flying',amount:1},{kind:'charge',amount:3}]};
 return <GameContext.Provider value={{state:{perspective:0,players:[{id:0,battlefield:[card]}]}}}>
  <button onClick={()=>setSick(!sick)}>Change sickness</button>
  <div style={{width:144,height:125,margin:60,'--bf-card-width':'144px','--bf-card-height':'125px'}}><GameCard card={card} variant="battlefield" style={{width:144,height:125}} /></div>
  <div id="shape-fixture" className="battlefield-row" style={{display:'flex',gap:30,padding:20,background:'#385d70'}}>
   {[
    {id:2,name:"Urza's Saga",type_line:'Enchantment — Saga',counters:[{kind:'lore',amount:1}]},
    {id:3,name:'Planeswalker',type_line:'Planeswalker',loyalty:3},
    {id:4,name:'Battle',type_line:'Battle',defense:4},
   ].map(shape=><GameCard key={shape.id} card={shape} variant="battlefield" className="hovered" style={{width:144,height:125}} />)}
  </div>
 </GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><HoverProvider><DragProvider><Fixture/></DragProvider></HoverProvider></I18nProvider>);
