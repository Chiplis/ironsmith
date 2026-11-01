import {setHandOpen} from '../src/lib/hand-open';
import React, {useEffect, useSyncExternalStore} from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {HoverProvider} from '../src/context/HoverContext';
import {I18nProvider} from '../src/i18n/I18nContext';
import {TooltipProvider} from '../src/components/ui/tooltip';
import DecisionPopupLayer from '../src/components/overlays/DecisionPopupLayer';
import HoverArtOverlay from '../src/components/right-rail/HoverArtOverlay';
import DiagnosticsSheet from '../src/components/layout/DiagnosticsSheet';
import PriorityPreferences from '../src/components/priority/PriorityPreferences';
import {createPriorityPolicy} from '../src/lib/priority-policy';
import {installPriorityShortcuts} from '../src/lib/priority-shortcuts';
import '../src/index.css';
const entry={name:'Ignoble Hierarch',controller:0,source_stable_id:10,ability_kind:'Triggered',automation_identity:'trigger:123',source_ability_text:'Exalted'};
const card={id:10,stable_id:10,name:entry.name,controller:0,owner:0,zone:'Battlefield',type_line:'Creature — Goblin Shaman',
 colors:['G'],mana_cost:'{G}',oracle_text:'Exalted\n{T}: Add {B}, {R}, or {G}.',power:'0',toughness:'1',ability_preferences:[entry]};
const state={snapshot_id:1,perspective:0,active_player:0,priority_player:0,phase:'FirstMain',stack_size:0,stack_objects:[],
 players:[{id:0,name:'Alice',battlefield:[card],hand_cards:[],mana_pool:{}},{id:1,name:'Bob',battlefield:[],hand_cards:[],mana_pool:{}}],
 decision:{kind:'priority',player:0,actions:[
 {index:0,kind:'pass_priority',label:'Keep hand',action_ref:{kind:'keep_opening_hand'}},
 {index:1,kind:'take_mulligan',label:'Mulligan',action_ref:{kind:'take_mulligan'}}]}};
const policy=createPriorityPolicy();
const game={objectDetails:async()=>card};
setHandOpen(true);
window.calls=[];window.policy=policy;window.fixtureState=state;
function App(){
 const revision=useSyncExternalStore(policy.subscribe,policy.getSnapshot);
 useEffect(()=>installPriorityShortcuts(document,{state:()=>state,maintain:()=>{},start:()=>{},clear:()=>{},group:()=>{},answer:()=>{}}),[]);
 return <I18nProvider><GameContext.Provider value={{state,game,matchClockStore:{subscribe:()=>()=>{},getSnapshot:()=>null},priorityPolicy:policy,priorityPolicyRevision:revision,
  dispatch:command=>window.calls.push(command),multiplayer:{mode:'idle'},holdRule:'never',setHoldRule:()=>{},playerAccentOverrides:{}}}>
 <HoverProvider><TooltipProvider>
 <div className="topbar-main-decision-host" style={{position:'relative',margin:40,width:650,height:60}}><DecisionPopupLayer priorityInline /></div>
 <div style={{position:'relative',width:700,height:700}}><HoverArtOverlay objectId={10} displayMode="card-frame" transientPreview={{card}} /></div>
 <PriorityPreferences/><DiagnosticsSheet/>
 </TooltipProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<App/>);
