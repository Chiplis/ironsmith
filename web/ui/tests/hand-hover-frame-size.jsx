import React from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import HandZone from "../src/components/board/HandZone";
import "../src/index.css";

// The app docks the hand at the bottom of the window inside .hand-reveal-shell,
// where the fan's scroller is overflow:visible. A hovered card has to grow to
// the frame renderer's size without leaving the window, so the fixture has to
// reproduce that geometry rather than float the hand in the middle of a page.
const card = {
  name: "Myr Moonvessel", controller: 0, owner: 0,
  type_line: "Artifact Creature — Myr", power: 1, toughness: 1,
  oracle_text: "When this creature dies, add {C}.", mana_cost: "{1}", power_toughness: "1/1", semantic_score: 1,
};

const context = {
  state: { perspective: 0, priority_player: 0, active_player: 0, stack: [], decision: window.__handDecision || { kind: "priority", player: 0 } },
  multiplayer: { mode: "idle" }, playerAccentOverrides: {}, game: null,
  dispatch: async () => {}, dispatchInBackground: async () => {},
};

function Fixture() {
  const hand = Array.from({ length: 7 }, (_, i) => ({ ...card, id: i + 1, stable_id: i + 1 }));
  const player = { id: 0, can_view_hand: true, hand_cards: hand };
  const state = { ...context.state, players: [player], snapshot_id: 1, zone_transitions: [] };
  return <I18nProvider><GameContext.Provider value={{ ...context, state }}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
    <main style={{ position: "fixed", inset: 0, overflow: "hidden" }}>
      <div style={{ position: "absolute", left: 0, right: 0, bottom: 0, height: 220 }} data-bottom-dock>
        <div className="hand-reveal-shell absolute left-1/2 bottom-0" data-open="true" style={{ height: 220 }}>
          <div className="hand-reveal-body" style={{ height: "100%" }}>
            <HandZone player={player} onInspect={() => {}} isExpanded layout="mobile-fan" />
          </div>
        </div>
      </div>
    </main>
  </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}

createRoot(document.getElementById("root")).render(<Fixture />);
