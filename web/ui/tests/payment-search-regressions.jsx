import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import DecisionPopupLayer from "../src/components/overlays/DecisionPopupLayer";
import "../src/index.css";

function Fixture() {
  const [search, setSearch] = useState(false);
  const [complete, setComplete] = useState(false);
  const [commands, setCommands] = useState([]);
  const candidates = [{ id: 10, name: "Island", legal: true }, { id: 11, name: "Swamp", legal: true }];
  const payment = { plan_id: complete ? "optimized" : "initial", request_hash: "request", planning_complete: complete, can_confirm: true, source_name: "Spell" };
  const decision = search
    ? {kind: "select_objects", player: 0, source_id: 1, description: "Search library", min: 0, max: 1, candidates}
    : {kind: "mana_payment", player: 0, source_id: 2, subject: "Spell", plan_id: payment.plan_id, request_hash: payment.request_hash};
  const state = { perspective: 0, players: [{id: 0, name: "Alice", battlefield: []}], decision, mana_payment: search ? null : payment,
    viewed_cards: search ? {visibility: "private", zone: "library", description: "Search library", card_ids: [10,11,12], cards: [...candidates, {id:12,name:"Lightning Bolt"}]} : null };
  return <GameContext.Provider value={{ state, multiplayer: {}, dispatch: command => setCommands(old => [...old, command]), dispatchInBackground: () => {} }}>
    <HoverProvider>
      <div style={{position: "absolute", bottom: 20, zIndex: 200}}>
      <button onClick={() => setComplete(true)}>Finish planning</button>
      <button onClick={() => setSearch(true)}>Fetch land</button>
      <output data-commands>{JSON.stringify(commands)}</output>
      </div>
      <DecisionPopupLayer priorityInline />
    </HoverProvider>
  </GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
