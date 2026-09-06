import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import TableCore from "../src/components/board/TableCore";
import Topbar from "../src/components/layout/TopBar";
import "../src/index.css";
const names = ["Ornithopter", "Myr Moonvessel", "Omniscience", "Mountain", "Forest", "Island", "Plains", "Swamp"];
const players = ["Alice", "Bob", "Charlie", "Diana"].map((name,id)=>({id,index:id,name,life:20,mana_pool:{},
  battlefield:names.map((name,i)=>({id:100*id+i+1,stable_id:100*id+i+1,name,controller:id,owner:id,lane:i<3?"creatures":"lands",type_line:i<3?"Artifact Creature":"Land",power:1,toughness:1,oracle_text:"",semantic_score:1})),
  hand_cards:[],graveyard_size:3,graveyard_cards:[{id:1000+id,name:"Plains"},{id:1100+id,name:"Mountain"},{id:1200+id,name:"Island"}],exile_cards:[{id:2000+id,name:"Swamp"}],command_cards:[],library_size:40,
}));
function Fixture(){
 const [result,setResult]=useState('none');
 const [targeting,setTargeting]=useState(false);
 const state={players,perspective:0,priority_player:0,active_player:0,decision:targeting?{kind:"targets",player:0,requirements:[{description:"Target card in a graveyard",min_targets:1,max_targets:1,legal_targets:[{kind:"object",object:1000},{kind:"object",object:1001}]}]}:{kind:"priority",player:0,actions:[{kind:"pass_priority",label:"Pass priority",index:0}]},stack:[9000],stack_objects:[{id:9000,name:"Lightning Bolt",controller:0,owner:0,type_line:"Instant",mana_cost:"{R}",targets:[]}],snapshot_id:1,phase:"Main",step:"Main1"};
 return <I18nProvider><GameContext.Provider value={{state,multiplayer:{mode:"idle"},playerAccentOverrides:{},game:null,holdRule:"never",setHoldRule:()=>{},dispatch:async()=>{},dispatchInBackground:async()=>{}}}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
 <main style={{height:"96vh"}}><button onClick={()=>setTargeting(true)}>Target graveyard cards</button><TableCore legalTargetObjectIds={targeting?new Set([1000,1001]):new Set()} onInspect={(id)=>setResult(String(id))} zoneViews={["battlefield"]} middleTopbar={<Topbar middleDocked />} zoneActionControls={<div className="table-zone-action-controls">{["Verify Match","Add Card","Compile Card","Load Decks","Puzzle Setup","Share Table","Create Lobby"].map(label=><button key={label} className="table-zone-action-button">{label}</button>)}</div>} /><output>{result}</output></main>
 </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
