import React from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import Topbar from "../src/components/layout/Topbar";
import "../src/index.css";

const clock = {enabled: true, remainingMsByPlayer: [2273000, 1801000], activePlayerIndex: 0};
const context = {
  state: {
    players: [{id: 0, index: 0, name: "Alice P1", life: 20, battlefield: [], hand_cards: []},
      {id: 1, index: 1, name: "Bob", life: 20, battlefield: [], hand_cards: []}],
    perspective: 0, priority_player: 0, active_player: 0, turn_number: 1,
    decision: {kind: "priority", player: 0, actions: []}, stack: [],
  },
  multiplayer: {mode: "host", matchStarted: true},
  playerAccentOverrides: {}, game: null,
  matchClockStore: {subscribe: () => () => {}, getSnapshot: () => clock},
  dispatch: async () => {}, dispatchInBackground: async () => {},
};
createRoot(document.getElementById("root")).render(
  <I18nProvider><GameContext.Provider value={context}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
    <Topbar />
  </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>
);
