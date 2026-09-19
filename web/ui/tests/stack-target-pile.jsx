import React from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { DragProvider } from "../src/context/DragContext";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import PlayerZonePiles from "../src/components/board/PlayerZonePiles";
import StackCard from "../src/components/cards/StackCard";
import "../src/index.css";

// A graveyard long enough to scroll, with the spell's target buried in it,
// and a second spell aimed at a battlefield permanent so the piles stay shut
// for it.
const graveyard = Array.from({ length: 16 }, (_, index) => ({ id: 40 - index, name: index % 2 ? "Island" : "Plains" }));
const player = {
  id: 0, name: "Alice", graveyard_size: graveyard.length, graveyard_cards: graveyard,
  exile_cards: [{ id: 60, name: "Swamp" }],
  battlefield: [{ id: 70, name: "Grizzly Bears", controller: 0 }],
};
const reanimate = { id: 200, inspect_object_id: 201, controller: 0, name: "Reanimate", mana_cost: "{B}", targets: [{ kind: "object", object: 30 }] };
const bolt = { id: 210, inspect_object_id: 211, controller: 0, name: "Lightning Bolt", mana_cost: "{R}", targets: [{ kind: "object", object: 70 }] };
const state = { players: [player], perspective: 0, stack_objects: [reanimate, bolt], decision: { kind: "priority", player: 0, actions: [] } };

function Fixture() {
  return <GameContext.Provider value={{ state, game: { objectDetails: async () => null }, dispatch: () => {} }}>
    <div data-stack-preview-anchor="true" style={{ position: "absolute", left: 20, top: 20, width: 240, display: "grid", gap: 6 }}>
      {[reanimate, bolt].map((entry) => <StackCard key={entry.id} entry={entry} />)}
    </div>
    <div style={{ position: "absolute", left: 300, top: 20, width: 640, height: 420 }}>
      <div className="has-zone-piles" style={{ height: 400, background: "#141414" }}>
        <PlayerZonePiles player={player} legalTargetObjectIds={new Set()} />
        <div className="battlefield-row" style={{ position: "relative", height: 380, paddingTop: 50 }}>
          <div className="battlefield-row-card" style={{ marginLeft: 160, width: 72, height: 100, background: "#776644" }}>Creature</div>
        </div>
      </div>
    </div>
  </GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><HoverProvider><DragProvider><Fixture /></DragProvider></HoverProvider></I18nProvider>);
