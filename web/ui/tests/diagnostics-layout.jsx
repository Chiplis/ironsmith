import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import TableCore from "../src/components/board/TableCore";
import Topbar from "../src/components/layout/Topbar";
import "../src/index.css";
const names = ["Ornithopter", "Myr Moonvessel", "Omniscience", "Mountain", "Forest", "Island", "Plains", "Swamp"];
const players = ["Alice", "Bob", "Charlie", "Diana"].map((name,id)=>({id,index:id,name,life:20,mana_pool:{},
  battlefield:names.map((name,i)=>({id:100*id+i+1,stable_id:100*id+i+1,name,controller:id,owner:id,lane:i<3?"creatures":"lands",type_line:i<3?"Artifact Creature":"Land",power:1,toughness:1,oracle_text:"",semantic_score:1})),
  hand_cards:[],graveyard_size:3,graveyard_cards:[{id:1000+id,name:"Plains",...(id===1?{counters:[{kind:"+1/+1",amount:2}]}:{})},{id:1100+id,name:"Mountain"},{id:1200+id,name:"Island"}],exile_cards:[{id:2000+id,name:"Swamp"}],command_cards:[],library_size:40,
}));
function Fixture(){
 const [result,setResult]=useState('none');
 const [holdRule,setHoldRule]=useState('never');
 const [autoResolveEnabled,setAutoResolveEnabled]=useState(false);
 const [expanded,setExpanded]=useState(true);
 const [targeting,setTargeting]=useState(true);
 const kind = new URLSearchParams(location.search).get('kind') || 'targets';
 const scenario = new URLSearchParams(location.search).get('scenario');
 const prevention = scenario === 'prevention';
 const longPayment = scenario === 'long-payment';
 if (longPayment) players[0].hand_cards = names.slice(0, 6).map((name, index) => ({ ...players[0].battlefield[index], id: 5000 + index, stable_id: 5000 + index, name }));
 const anyTarget = prevention || scenario === 'any-target';
 const decisions = {
 priority: {kind:'priority',player:0,actions:[{kind:'pass_priority',label:'Pass priority',index:0,action_ref:{kind:'pass_priority'}}]},
 targets: {kind:'targets',player:0,description: anyTarget && !prevention ? 'It deals 2 damage to any target.' : undefined, context_text: prevention ? '{W}, Sacrifice this creature: Prevent the next 2 damage that would be dealt to any target this turn.' : undefined, requirements:[{description:prevention ? 'target to protect' : anyTarget ? 'Any target' : 'Target card',min_targets:1,max_targets:1,legal_targets:anyTarget ? players.flatMap(player => [{kind:'player',player:player.id,name:player.name}, ...player.battlefield.filter(card => card.lane === 'creatures').map(card => ({kind:'object',object:card.id,name:card.name}))]) : [{kind:'object',object:1000}]}]},
 select_objects: {kind:'select_objects',player:0,description:'Scry 20 — select cards to put on bottom of library',min:0,max:3,candidates:Array.from({length:20},(_,index)=>({id:3000+index,name:'Long candidate card name '+index,object_controller:0,legal:true}))},
 select_options: {kind:'select_options',player:0,description:'Choose cards',min:0,max:3,options:Array.from({length:20},(_,index)=>({index,description:'Draw cards and return a creature from your graveyard to your hand. Option '+index,legal:true}))},
 mana_payment: {kind:'mana_payment',player:0,description:'Pay {1}{G}'},
 attackers: {kind:'attackers',player:0,attacker_options:[{creature:1,name:'Ornithopter',valid_targets:[{kind:'player',player:1}]}]},
 };

 if (scenario === 'optional-target') {
   decisions.targets = {
     kind: 'targets', player: 0,
     source_id: 1, source_name: 'Yawgmoth, Thran Physician',
     context_text: 'Pay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.',
     requirements: [{description: 'target creature for counters', min_targets: 0, max_targets: 1,
       legal_targets: players.flatMap(player => player.battlefield.filter(card => card.lane === 'creatures').map(card => ({kind: 'object', object: card.id, name: card.name}))) }],
   };
 }

 const state={cancelable:scenario === 'optional-target',players,perspective:0,priority_player:0,active_player:0,decision: expanded ? decisions[kind] : null, mana_payment: kind === 'mana_payment' ? {source_name:'Grizzly Bears',can_confirm:true,planning_complete:true,request_hash:'test',plan_id:'test',pips:longPayment ? [['14']] : [['1'],['G']],pool_before:{green:longPayment ? 14 : 2},pool_after_activations:{green:longPayment ? 14 : 2},pool_after_payment:{},planned_sources:[],available_sources:[],allocations:[],warnings:[],life_to_pay:0} : null,stack:[9000],stack_objects:[{id:9000,name:"Lightning Bolt",controller:0,owner:0,type_line:"Instant",mana_cost:"{R}",targets:[]}],snapshot_id:1,phase:"Main",step:"Main1"};
 return <I18nProvider><GameContext.Provider value={{state,matchClockStore:{subscribe:()=>()=>{},getSnapshot:()=>null},multiplayer:{mode:"idle"},playerAccentOverrides:{},game:null,cancelDecision:async()=>{},holdRule,setHoldRule,autoResolveEnabled,setAutoResolveEnabled,dispatch:async(action)=>{window.__dispatched=action;},dispatchInBackground:async()=>{}}}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
 <main style={{height:"96vh"}}><button onClick={()=>setExpanded(value=>!value)}>Toggle decision</button><button onClick={()=>setTargeting(true)}>Target graveyard cards</button><TableCore legalTargetObjectIds={targeting?new Set([1000,1001]):new Set()} onInspect={(id)=>setResult(String(id))} zoneViews={["battlefield"]} middleTopbar={<Topbar middleDocked />} middleUtilityControls={<div className="topbar-minor-controls--utility" />} zoneActionControls={<div className="table-zone-action-controls">{["Verify Match","Add Card","Compile Card","Load Decks","Puzzle Setup","Share Table","Create Lobby"].map(label=><button key={label} className="table-zone-action-button">{label}</button>)}</div>} /><output>{result}</output></main>
 </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
