import React, { useLayoutEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { ObjectSelectionProvider } from "../src/context/ObjectSelectionContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import DecisionPopupLayer from "../src/components/overlays/DecisionPopupLayer";
import PlayerZonePiles from "../src/components/board/PlayerZonePiles";
import { LOOK_DONE_EVENT } from "../src/lib/look-pile";
import "../src/index.css";

const cards = [{ id: 1, name: "Island" }, { id: 2, name: "Plains" }, { id: 3, name: "Forest" }];
const params = new URLSearchParams(location.search);
const mechanic = params.has("surveil") ? "Surveil" : params.has("choose") ? "Choose a card" : "Scry";
const view = { acknowledged: false, viewer: 0, subject: 0, zone: params.has("choose") ? "hand" : "library", visibility: "private", source: 10,
  description: `${mechanic} 3 card(s)`, card_ids: cards.map(card => card.id), cards };
const priority = { kind: "priority", player: 0,
  actions: [{ index: 0, kind: "pass_priority", label: "Pass priority" }] };
const initialState = { snapshot_id: 1, perspective: 0, active_player: 0, priority_player: 0,
  phase: "Beginning", step: "Upkeep", stack_size: 0,
  players: [0, 1].map(id => ({ id, name: `Player ${id}`, battlefield: [], hand_cards: [], mana_pool: {} })),
  viewed_cards: view,
  decision: { kind: "select_objects", player: 0, source_id: 10, source_name: "Sphinx of Foresight",
    reason: mechanic, description: `${mechanic} 3 — select cards to put on ${mechanic === "Surveil" ? "graveyard" : "bottom"}`, min: 0, max: 3,
    candidates: cards.map(card => ({ ...card, legal: true })) } };

function Fixture() {
  const [state, setState] = useState(initialState);
  useLayoutEffect(() => { window.__fixtureState = state; }, [state]);
  useLayoutEffect(() => {
    window.__publishSnapshot = setState;
    window.__lookDoneCount = 0;
    const done = () => { window.__lookDoneCount += 1; };
    window.addEventListener(LOOK_DONE_EVENT, done);
    return () => window.removeEventListener(LOOK_DONE_EVENT, done);
  }, []);
  const dispatch = command => setState(current => ({ ...current, snapshot_id: current.snapshot_id + 1,
    viewed_cards: { ...current.viewed_cards, acknowledged: true },
    decision: command.type === "select_objects" ? {
      kind: "select_options", player: 0, source_id: 10, source_name: "Sphinx of Foresight",
      reason: "Ordering", description: "Reorder cards to keep on top of your library", min: 3, max: 3,
      options: cards.map((card, index) => ({ index, description: card.name, object_id: card.id, legal: true })),
    } : priority,
  }));
  return <I18nProvider><GameContext.Provider value={{ state, dispatch, game: null,
    multiplayer: { mode: "idle" }, holdRule: "never", setHoldRule: () => {}, playerAccentOverrides: {} }}>
    <ObjectSelectionProvider><HoverProvider><TooltipProvider>
      <div className="has-zone-piles" style={{ position: "relative", width: 600, height: 200 }}>
        <PlayerZonePiles player={state.players[0]} />
      </div>
      <div style={{ position: "relative", margin: 40, width: 600, height: 300 }}>
        <DecisionPopupLayer priorityInline mobileBattle={new URLSearchParams(location.search).has("mobile")} />
      </div>
    </TooltipProvider></HoverProvider></ObjectSelectionProvider>
  </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById("root")).render(<Fixture />);
