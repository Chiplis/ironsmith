import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { ObjectSelectionProvider } from "../src/context/ObjectSelectionContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import SelectObjectsDecision from "../src/components/decisions/SelectObjectsDecision";
import PlayerZonePiles from "../src/components/board/PlayerZonePiles";
import GameCard from "../src/components/cards/GameCard";
import { requestObjectSelection } from "../src/lib/object-selection";
import "../src/index.css";

const fieldCard = {id: 10, name: "Llanowar Elves", controller: 0, owner: 0, type_line: "Creature — Elf Druid", power: 1, toughness: 1, power_toughness: "1/1"};
const graveyardCards = [{id: 20, name: "Island"}, {id: 21, name: "Swamp"}];
const decision = {kind: "select_objects", player: 0, min: 0, max: 2, description: "Search your library", candidates: [
  {id: 10, name: "Llanowar Elves", legal: true},
  {id: 20, name: "Island", legal: true},
  {id: 21, name: "Swamp", legal: true},
]};

function Fixture() {
  const [commands, setCommands] = useState([]);
  const player = {id: 0, index: 0, name: "Alice", battlefield: [fieldCard], graveyard_size: graveyardCards.length, graveyard_cards: graveyardCards, exile_cards: []};
  const viewed_cards = new URLSearchParams(window.location.search).has("look") ? { cards: graveyardCards } : null;
  const state = {viewed_cards, players: [player], perspective: 0, priority_player: 0, active_player: 0, decision, stack: []};
  const context = {state, multiplayer: {mode: "idle"}, playerAccentOverrides: {}, game: null,
    dispatch: (command) => setCommands(old => [...old, command]), dispatchInBackground: () => {}};
  return <GameContext.Provider value={context}><ObjectSelectionProvider><HoverProvider><DragProvider><TooltipProvider>
    <main style={{padding: 20}}>
      <output data-commands>{JSON.stringify(commands)}</output>
      <SelectObjectsDecision decision={decision} canAct layout="strip" />
      <div className="has-zone-piles" style={{position: "relative", height: 320, background: "#141414"}}>
        <PlayerZonePiles player={player} legalTargetObjectIds={new Set()} onCardClick={() => {}} />
        <div className="battlefield-row" style={{position: "relative", height: 300, paddingTop: 40, paddingLeft: 160}}>
          <div className="battlefield-row-card" style={{width: 124}}>
            <GameCard card={fieldCard} onClick={() => requestObjectSelection(fieldCard.id, "add")} />
          </div>
        </div>
      </div>
    </main>
  </TooltipProvider></DragProvider></HoverProvider></ObjectSelectionProvider></GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
