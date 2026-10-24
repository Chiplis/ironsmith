import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import { buildEffectOrderingKey } from "../src/lib/effect-ordering";
import useCardStackMode from "../src/hooks/useCardStackMode";
import CardStackPile from "../src/components/right-rail/CardStackPile";
import TableCore from "../src/components/board/TableCore";
import TopbarUtilityControls from "../src/components/layout/TopbarUtilityControls";
import Topbar from "../src/components/layout/Topbar";
import "../src/index.css";
const names = ["Ornithopter", "Myr Moonvessel", "Omniscience", "Mountain", "Forest", "Island", "Plains", "Swamp"];
const sizingParams = new URLSearchParams(location.search);
const sizingPlayers = Math.max(2, Math.min(4, Number(sizingParams.get('players')) || 4));
const players = ["Alice", "Bob", "Charlie", "Diana"].slice(0, sizingPlayers).map((name,id)=>({id,index:id,name,life:20,mana_pool:{},
  battlefield:names.map((name,i)=>({id:100*id+i+1,stable_id:100*id+i+1,name,controller:id,owner:id,lane:i<3?"creatures":"lands",type_line:i<3?"Artifact Creature":"Land",power:1,toughness:1,oracle_text:"",semantic_score:1})),
  hand_cards:[],graveyard_size:3,graveyard_cards:[{id:1000+id,name:"Plains",...(id===1?{counters:[{kind:"+1/+1",amount:2}]}:{})},{id:1100+id,name:"Mountain"},{id:1200+id,name:"Island"}],exile_cards:[{id:2000+id,name:"Swamp"}],command_cards:[],library_size:40,
}));
if (sizingParams.has('objects')) {
  for (const player of players) {
    const count = Number(sizingParams.get(player.id === 1 ? 'opponentObjects' : 'objects') || sizingParams.get('objects'));
    player.battlefield = Array.from({length: count}, (_, i) => ({
      ...player.battlefield[i % names.length], id: player.id * 1000 + i + 1,
      stable_id: player.id * 1000 + i + 1, name: `Object ${i}`, lane: 'creatures',
    }));
  }
}
function StackFixture(){
 const enabled = useCardStackMode();
 return enabled ? <CardStackPile /> : null;
}
function Fixture(){
 const [stackOrder,setStackOrder]=useState(sizingParams.get("stackOrder"));
 useEffect(()=>{window.setStackOrder=setStackOrder;},[]);
 const [stackCount,setStackCount]=useState(1);
 useEffect(()=>{window.setStackCount=setStackCount;},[]);
 const [stackLaneVisible,setStackLaneVisible]=useState(false);
 useEffect(()=>{window.setStackLaneVisible=setStackLaneVisible;},[]);
 const [result,setResult]=useState('none');
 const [holdRule,setHoldRule]=useState('never');
 const [autoResolveEnabled,setAutoResolveEnabled]=useState(false);
 const [phasePassing,setPhasePassing]=useState(false);
 const [priorityStops,setPriorityStops]=useState({"phase:Main2":"once"});
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

 const auditOptions = ['White', 'Blue', 'Black', 'Red', 'Green'].map((description,index)=>({index,description,legal:true,repeatable:true,max_count:2}));
 if (scenario === 'mana-colors') decisions.select_options = {kind:'select_options',player:0,source_name:'Manamorphose',description:'Add two mana in any combination of colors',reason:'Choose color',min:1,max:2,options:auditOptions};
 if (scenario === 'distribute') decisions.select_options = {kind:'select_options',player:0,description:'Distribute 3 damage among targets',reason:'distribute',min:3,max:3,options:auditOptions.slice(0,3).map((opt,index)=>({...opt,description:['Alice','Bob','Ornithopter'][index],max_count:3}))};
 if (scenario === 'counters') decisions.select_options = {kind:'select_options',player:0,description:'Choose counters',reason:'counters',min:0,max:3,options:[{index:0,description:'+1/+1 counters',legal:true,max_count:3},{index:1,description:'Charge counters',legal:true,max_count:3}]};
 if (scenario === 'ordering') decisions.select_options = {kind:'select_options',player:0,description:'Order cards',reason:'ordering',min:3,max:3,options:['Island','Mountain','Forest'].map((description,index)=>({index,description,legal:true}))};
 if (scenario === 'single-color') decisions.select_options = {kind:'select_options',player:0,description:'Choose a color',reason:'Choose color',min:1,max:1,options:auditOptions.map(opt=>({...opt,repeatable:false}))};
 decisions.number = {kind:'number',player:0,description:'Choose a value for X',min:0,max:20};
 decisions.text_input = {kind:'text_input',player:0,description:'Name a card'};

 if (scenario === 'optional-target') {
   decisions.targets = {
     kind: 'targets', player: 0,
     source_id: 1, source_name: 'Yawgmoth, Thran Physician',
     context_text: 'Pay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.',
     requirements: [{description: 'target creature for counters', min_targets: 0, max_targets: 1,
       legal_targets: players.flatMap(player => player.battlefield.filter(card => card.lane === 'creatures').map(card => ({kind: 'object', object: card.id, name: card.name}))) }],
   };
 }

 const state={cancelable:scenario === 'optional-target',players,perspective:0,priority_player:0,active_player:scenario === 'opponent-turn' ? 1 : 0,decision: expanded ? decisions[kind] : null, mana_payment: kind === 'mana_payment' ? {source_name:'Grizzly Bears',can_confirm:true,planning_complete:true,request_hash:'test',plan_id:'test',pips:longPayment ? [['14']] : [['1'],['G']],pool_before:{green:longPayment ? 14 : 2},pool_after_activations:{green:longPayment ? 14 : 2},pool_after_payment:{},planned_sources:[],available_sources:[],allocations:[],warnings:[],life_to_pay:0} : null,stack:[9000],stack_objects:[{id:9000,name:"Lightning Bolt",controller:0,owner:0,type_line:"Instant",mana_cost:"{R}",targets:[]}],snapshot_id:1,phase:"Main",step:"Main1"};
 if (scenario === 'progenitus') {
   const colors = ['white','white','blue','blue','black','black','red','red','green','green'];
   const lands = ['Plains','Plains','Island','Island','Swamp','Swamp','Mountain','Mountain','Forest','Forest'];
   const sources = colors.map((color,index)=>({source_id:String(6000+index),source_name:lands[index],ability_index:0,expected_mana:{[color]:1}}));
   players[0].battlefield = sources.map(source=>({id:Number(source.source_id),stable_id:Number(source.source_id),name:source.source_name,controller:0,owner:0,lane:'lands',type_line:'Basic Land',oracle_text:''}));
   if (kind === 'mana_payment') {
     state.decision = {...decisions.mana_payment,source_name:'Progenitus',description:'Pay {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}'};
     state.mana_payment = {...state.mana_payment,source_name:'Progenitus',pips:['W','W','U','U','B','B','R','R','G','G'].map(color=>[color]),pool_before:{},pool_after_activations:{white:2,blue:2,black:2,red:2,green:2},planned_sources:sources,activation_options:[...sources,{source_id:'6100',source_name:'City of Brass',ability_index:0,expected_mana:{white:1}}],activation_options_complete:true};
   }
 }
 if (scenario === 'kozilek-command' && kind === 'mana_payment') {
   const sources = [
     {source_id:'6000',source_name:'Eldrazi Temple',ability_index:1,expected_mana:{colorless:2}},
     {source_id:'6001',source_name:"Ugin's Labyrinth",ability_index:1,expected_mana:{colorless:2}},
   ];
   players[0].battlefield = sources.map(source=>({id:Number(source.source_id),stable_id:Number(source.source_id),name:source.source_name,controller:0,owner:0,lane:'lands',type_line:'Land',oracle_text:''}));
   players[0].battlefield.push({id:6002,stable_id:6002,name:'Glaring Fleshraker',controller:0,owner:0,lane:'creatures',type_line:'Creature — Eldrazi Drone',power:2,toughness:2});
   state.decision = {...decisions.mana_payment,source_name:"Kozilek's Command",description:'Pay {C}{C}{1}{1}'};
   state.mana_payment = {...state.mana_payment,source_name:"Kozilek's Command",pips:[['C'],['C'],['1'],['1']],cost_context:['Normal cost','X = 2'],pool_before:{},pool_after_activations:{colorless:4},planned_sources:sources,activation_options:[...sources,{source_id:'6003',source_name:'Wastes',ability_index:0,expected_mana:{colorless:1}}],activation_options_complete:true};
 }
 if (state.mana_payment && sizingParams.get('paymentReady') === 'false') state.mana_payment.can_confirm = false;
 if (sizingParams.has("stackLaneTest")) {
   state.stack_objects = Array.from({length:stackCount},(_,index)=>({...state.stack_objects[0],id:9000+index}));
   state.stack = state.stack_objects.map(entry=>entry.id);
   state.stack_size = state.stack_objects.length;
 }
 if (scenario === "phase-controls" && !stackLaneVisible) { state.phase = "FirstMain"; state.step = null; state.stack = []; state.stack_objects = []; state.stack_size = 0; }
 if (stackOrder) state.decision = {
   kind: "select_options", player: 0,
   reason: stackOrder === "replacement" ? "Replacement effect" : "Order triggers",
   description: stackOrder === "replacement" ? "Choose a replacement effect" : "Order triggered abilities",
   min: stackOrder === "replacement" ? 1 : 2, max: stackOrder === "replacement" ? 1 : 2,
   options: [0,1].map(index=>({index,description:`Object ${index}\nWhenever you cast a noncreature spell, this ability triggers.`,related_object_ids:[index+1],legal:true})),
 };
 const effectOrderingState = stackOrder ? {key:buildEffectOrderingKey(state.decision),order:[0,1]} : null;
 return <I18nProvider><GameContext.Provider value={{state,effectOrderingState,matchClockStore:{subscribe:()=>()=>{},getSnapshot:()=>null},multiplayer:{mode:"idle"},playerAccentOverrides:{},game:null,cancelDecision:async()=>{window.__cancelled=true;},holdRule,setHoldRule,autoResolveEnabled,setAutoResolveEnabled,phasePassing,priorityStops,cyclePriorityStop:(key)=>setPriorityStops(stops=>({...stops,[key]:stops[key]?undefined:"once"})),startPhaseAdvance:()=>{window.__phaseAdvance=true;setPhasePassing(true);},togglePhasePassing:()=>setPhasePassing(value=>!value),dispatch:async(action)=>{window.__dispatched=action;},dispatchInBackground:async()=>{}}}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
 {sizingParams.has("stackLaneTest") && <StackFixture />}
 <main style={{height:"96vh"}}><button onClick={()=>setExpanded(value=>!value)}>Toggle decision</button><button onClick={()=>setTargeting(true)}>Target graveyard cards</button><TableCore onChangePerspective={id=>{window.selectedPerspective=id;}} legalTargetObjectIds={targeting?new Set([1000,1001]):new Set()} onInspect={(id)=>setResult(String(id))} zoneViews={["battlefield"]} middleTopbar={<Topbar middleDocked />} middleUtilityControls={<TopbarUtilityControls />} zoneActionControls={<div className="table-zone-action-controls">{["Verify Match","Add Card","Compile Card","Load Decks","Puzzle Setup","Share Table","Create Lobby"].map(label=><button key={label} className="table-zone-action-button">{label}</button>)}</div>} /><output>{result}</output></main>
 </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
