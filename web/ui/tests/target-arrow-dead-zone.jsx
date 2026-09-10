import { useEffect } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { useCombatArrows } from "../src/context/useCombatArrows";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import TableCore from "../src/components/board/TableCore";
import "../src/index.css";

const creature = (id, controller) => ({
  id,
  stable_id: id,
  name: controller === 0 ? "Grizzly Bears" : "Goblin Piker",
  controller,
  owner: controller,
  lane: "creatures",
  type_line: "Creature — Bear",
  power: 2,
  toughness: 2,
  oracle_text: "",
  semantic_score: 1,
});

const decision = {
  kind: "targets",
  player: 0,
  source_id: 10,
  description: "Choose targets for Lightning Bolt",
  requirements: [{
    description: "any target",
    min_targets: 1,
    max_targets: 1,
    legal_targets: [{ kind: "object", object: 20, name: "Goblin Piker" }],
  }],
};

const state = {
  perspective: 0,
  priority_player: 0,
  active_player: 0,
  snapshot_id: 1,
  phase: "Main",
  step: "Main",
  cancelable: true,
  decision,
  stack: [],
  players: [0, 1].map((id) => ({
    id,
    index: id,
    name: id ? "Bob" : "Alice",
    life: 20,
    battlefield: [creature(id === 0 ? 10 : 20, id)],
    hand_cards: [],
    graveyard_cards: [],
    exile_cards: [],
    command_cards: [],
    mana_pool: {},
  })),
};

// The arrow's tip is context state, so the probe reads it there. Every tip is
// kept: a driven mouse cannot rest anywhere without moving there first, so the
// tip the arrow started with is only observable in the history.
window.__dragArrowHistory = [];

export function ArrowProbe() {
  const { dragArrow } = useCombatArrows();
  useEffect(() => {
    window.__dragArrow = dragArrow ? { ...dragArrow } : null;
    if (dragArrow) window.__dragArrowHistory.push({ ...dragArrow });
  }, [dragArrow]);
  return null;
}

export function Fixture() {
  return (
    <I18nProvider>
      <GameContext.Provider value={{
        state,
        multiplayer: { mode: "idle" },
        // The diagnostics sheet inside the table subscribes to the clock.
        matchClockStore: { subscribe: () => () => {}, getSnapshot: () => null },
        playerAccentOverrides: {},
        game: null,
        holdRule: "never",
        setHoldRule: () => {},
        cancelDecision: () => {},
        dispatch: async (command) => { window.__dispatched = command; },
        dispatchInBackground: async () => {},
      }}>
        <HoverProvider>
          <DragProvider>
            <CombatArrowProvider>
              <TooltipProvider>
                <ArrowProbe />
                <main style={{ height: "96vh" }}>
                  <TableCore
                    zoneViews={[]}
                    onInspect={() => {}}
                    middleTopbar={<div style={{ height: 60 }}><div className="topbar-main-decision-host" data-topbar-main-decision-host="true" /></div>}
                  />
                </main>
              </TooltipProvider>
            </CombatArrowProvider>
          </DragProvider>
        </HoverProvider>
      </GameContext.Provider>
    </I18nProvider>
  );
}

createRoot(document.getElementById("root")).render(<Fixture />);
