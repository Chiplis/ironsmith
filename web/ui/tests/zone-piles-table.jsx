import TopbarUtilityControls from "../src/components/layout/TopbarUtilityControls";
import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import HandZone from "../src/components/board/HandZone";
import TableCore from "../src/components/board/TableCore";
import Topbar from "../src/components/layout/TopBar";
import FloatingCardPreview from "../src/components/right-rail/FloatingCardPreview";
import CardStackPile from "../src/components/right-rail/CardStackPile";
import "../src/index.css";
import useBattlefieldAppearance from "../src/hooks/useBattlefieldAppearance";
const names = ["Ornithopter", "Myr Moonvessel", "Omniscience", "Mountain", "Forest", "Island", "Plains", "Swamp"];
const players = ["Alice", "Bob", "Charlie", "Diana"].map((name,id)=>({id,index:id,name,life:20,mana_pool:{},
  battlefield:names.map((name,i)=>({id:100*id+i+1,stable_id:100*id+i+1,name,controller:id,owner:id,lane:i<3?"creatures":"lands",type_line:i<3?"Artifact Creature":"Land",power:1,toughness:1,oracle_text:"",semantic_score:1})),
  can_view_hand: true, hand_cards: new URLSearchParams(location.search).has("decisionDock") && id === 0 ? Array.from({length:Number(new URLSearchParams(location.search).get("hand") || 4)},(_,i)=>({id:8000+i,name:names[i % names.length]})) : [],graveyard_size:3,graveyard_cards:[{id:1000+id,name:"Plains"},{id:1100+id,name:"Mountain"},{id:1200+id,name:"Island"}],exile_cards:[{id:2000+id,name:"Swamp"}],command_cards: new URLSearchParams(location.search).has("commander") ? [{id:3000+id,name:"Commander"}] : [],library_size:40,
}));
function Fixture(){
 const targetLook = new URLSearchParams(location.search).has("targetLook");
 const [appearance,setAppearance]=useBattlefieldAppearance();
 const [result,setResult]=useState('none');
 const [perspective,setPerspective]=useState(0);
 const multiplayer = new URLSearchParams(location.search).has('multiplayer') ? {mode:'in_match',role:'client'} : {mode:'idle'};
 const [targeting,setTargeting]=useState(false);
 const state={viewed_cards: new URLSearchParams(location.search).has("look") ? {cards:[{id:7001,name:"Island"},{id:7002,name:"Forest"}]} : undefined,players,perspective,priority_player:0,active_player:0,decision:targeting?{kind:"targets",player:0,requirements:[{description:"Target card in a graveyard",min_targets:1,max_targets:1,legal_targets:[{kind:"object",object:1000},{kind:"object",object:1001}]}]}:{kind:"priority",player:0,actions:[{kind:"pass_priority",label:"Pass priority",index:0}]},stack:[9000],stack_objects:[{id:9000,name:"Lightning Bolt",controller:0,owner:0,type_line:"Instant",mana_cost:"{R}",targets:[]}],snapshot_id:1,phase:"Main",step:"Main1"};
 if (new URLSearchParams(location.search).has("decisionDock")) {
   state.decision = {kind:"targets",player:0,description:"Target player creates X 0/1 colorless Eldrazi Spawn creature tokens.",requirements:Array.from({length:2},(_,i)=>({description:i ? "Target player scries X, then draws a card." : "Target player creates X tokens.",min_targets:1,max_targets:1,legal_targets:Array.from({length:Number(new URLSearchParams(location.search).get("options") || 2)},(_,j)=>({kind:"player",player:j,name:players[j]?.name || `Player ${j}`}))}))};
 }
 if (new URLSearchParams(location.search).has("objects")) {
   state.players = players.slice(0,2);
   state.decision = {kind:"select_objects",player:0,min:1,max:1,description:"Choose a permanent",candidates:Array.from({length:30},(_,i)=>({id:10000+i,name:`Permanent ${i+1}`,legal:true}))};
 }
 return <I18nProvider><GameContext.Provider value={{state,matchClockStore:{subscribe:()=>()=>{},getSnapshot:()=>null},multiplayer,playerAccentOverrides:{},game:null,holdRule:"never",setHoldRule:()=>{},dispatch:async()=>{},dispatchInBackground:async()=>{}}}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
 <main style={{height:"96vh"}}><button onClick={()=>setAppearance("compactCards",!appearance.compactCards)}>Toggle battlefield style</button><button onClick={()=>setTargeting(true)}>Target graveyard cards</button><TableCore middleUtilityControls={new URLSearchParams(location.search).has("utilities") ? <TopbarUtilityControls playerNames={["Alice", "Bob"]} startingLife={20} /> : null} onChangePerspective={setPerspective} legalTargetObjectIds={targetLook ? new Set([7001,7002]) : targeting?new Set([1000,1001]):new Set()} onInspect={(id)=>setResult(String(id))} zoneViews={["battlefield"]} middleTopbar={<Topbar middleDocked />} zoneActionControls={<div className="table-zone-action-controls">{["Verify Match","Add Card","Compile Card","Load Decks","Puzzle Setup","Share Table","Create Lobby"].map(label=><button key={label} className="table-zone-action-button">{label}</button>)}</div>} /><output>{result}</output></main>
 {new URLSearchParams(location.search).has("decisionDock") && <div data-bottom-dock style={{position:'fixed',left:0,right:0,bottom:8,height:96,pointerEvents:'none'}}><div className="hand-reveal-shell" data-open="false" style={{position:'absolute',left:'50%',bottom:0,height:96}}><div className="hand-reveal-body" style={{height:'100%'}}><HandZone player={players[0]} isExpanded={false} layout="mobile-fan" /></div></div></div>}
 <FloatingCardPreview />
 {appearance.compactCards ? <CardStackPile onInspectObject={(id)=>setResult(String(id))} /> : null}
 </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
