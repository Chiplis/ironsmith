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
import { useCombatArrows } from "../src/context/useCombatArrows";
import "../src/index.css";

const cards = [10, 20].map((id) => ({id, name: "Myr Moonvessel", controller: id === 10 ? 0 : 1, owner: id === 10 ? 0 : 1, type_line: "Artifact Creature — Myr", power: 1, toughness: 1, oracle_text: "", semantic_score: 1}));
const decision = {kind: "select_objects", player: 0, min: 1, max: 1, description: "Choose a creature to sacrifice", candidates: [{id: 10, name: "Myr Moonvessel", legal: true}]};
const context = {
  state: {players: cards.map((card, id) => ({id, index:id, name:id ? "Bob" : "Alice", battlefield:[card]})), perspective:0, priority_player:0, active_player:0, decision, stack:[]},
  multiplayer: {mode:"idle"}, playerAccentOverrides: {}, game:null,
  dispatch:async()=>{}, dispatchInBackground:async()=>{},
};
export function DefenderTarget() {
  const { combatMode } = useCombatArrows();
  return <button onClick={()=>combatMode?.onTargetAreaClick?.(1, null)}>Defend Bob</button>;
}
export function CombatDecisionFixture() {
  const [opponent, setOpponent] = useState(false);
  const [started, setStarted] = useState(false);
  const [result, setResult] = useState("");
  const battlefield = [{...cards[0], id:10, stable_id:10, lane:"creatures", controller:0}];
  const combatDecision = {kind:"attackers", player:0, attacker_options:[{creature:10, creature_name:"Myr Moonvessel", valid_targets:[{Player:1}]}]};
  const gameState = {...context.state, decision:started ? combatDecision : {kind:"priority",player:0,actions:[{kind:"pass_priority",label:"Pass priority",action_index:0}]}, players:context.state.players.map((player,index)=>({...player,life:20,battlefield:index===0?battlefield:[], hand_cards:[],graveyard_size:2,graveyard_cards:[{id:101,name:"Ornithopter"},{id:102,name:"Lightning Bolt"}],exile_cards:[{id:103,name:"Swamp"}],command_cards:[],mana_pool:{}})), snapshot_id:1, perspective:opponent ? 1 : 0, phase:"Combat", step:"DeclareAttackers", cancelable:true};
  return <I18nProvider><GameContext.Provider value={{...context, state:gameState, holdRule:"never", setHoldRule:()=>{}, cancelDecision:()=>setResult("cancelled"), dispatch:async action=>setResult(JSON.stringify(action))}}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
    <main style={{height:"90vh"}}>
      <button onClick={()=>setStarted(true)}>Go to Attackers</button>
      <TableCore zoneActionControls={<div className="table-zone-action-controls">{["Verify Match", "Add Card", "Compile Card", "Load Decks", "Puzzle Setup", "Share Table", "Create Lobby"].map(label=><button key={label} className="table-zone-action-button">{label}</button>)}</div>} zoneViews={["graveyard", "exile"]} onInspect={()=>{}} middleTopbar={<Topbar middleDocked />} />
      <DefenderTarget /><button onClick={()=>setOpponent(value=>!value)}>Switch perspective</button>
      <output>{result}</output>
    </main>
  </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById("root")).render(<CombatDecisionFixture />);
