import React from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import SelectObjectsDecision from "../src/components/decisions/SelectObjectsDecision";
import BattlefieldRow from "../src/components/board/BattlefieldRow";
import "../src/index.css";

const cards = [10, 20].map((id) => ({id, name: "Myr Moonvessel", controller: id === 10 ? 0 : 1, owner: id === 10 ? 0 : 1, type_line: "Artifact Creature — Myr", power: 1, toughness: 1, oracle_text: "", semantic_score: 1}));
const decision = {kind: "select_objects", player: 0, min: 1, max: 1, description: "Choose a creature to sacrifice", candidates: [{id: 10, name: "Myr Moonvessel", legal: true}]};
const context = {
  state: {players: cards.map((card, id) => ({id, index:id, name:id ? "Bob" : "Alice", battlefield:[card]})), perspective:0, priority_player:0, active_player:0, decision, stack:[]},
  multiplayer: {mode:"idle"}, playerAccentOverrides: {}, game:null,
  dispatch:async()=>{}, dispatchInBackground:async()=>{},
};
createRoot(document.getElementById("root")).render(
  <I18nProvider><GameContext.Provider value={context}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
    <main style={{padding:40}}>
      <SelectObjectsDecision decision={decision} canAct layout="strip" inlineSubmit={false} />
      <div style={{height:350, marginTop:50}}><BattlefieldRow cards={cards} onInspect={()=>{}} activatableMap={new Map()} /></div>
      <button type="button">Outside decision</button>
    </main>
  </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>
);
