import { useEffect } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider, useHover } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import TargetsDecision from "../src/components/decisions/TargetsDecision";
import RightRail from "../src/components/right-rail/RightRail";
import FloatingCardPreview from "../src/components/right-rail/FloatingCardPreview";
import StackCard from "../src/components/cards/StackCard";
import "../src/index.css";

const params = new URLSearchParams(window.location.search);
const scenario = params.get("scenario") || "forced-opponent";
// Workspace mounts every inspector dock with allowHoverFallback={false}.
const allowHoverFallback = params.get("hoverFallback") === "1";

const REQUIREMENTS = {
  // "Target opponent discards a card" with one opponent: no decision to make.
  "forced-opponent": [{
    description: "target opponent",
    min_targets: 1,
    max_targets: 1,
    legal_targets: [{ kind: "player", player: 1, name: "Bob" }],
  }],
  // Three players: the opponent is a real choice, so nothing is picked.
  "two-opponents": [{
    description: "target opponent",
    min_targets: 1,
    max_targets: 1,
    legal_targets: [
      { kind: "player", player: 1, name: "Bob" },
      { kind: "player", player: 2, name: "Carol" },
    ],
  }],
  // A single legal creature is just as forced as a single legal player.
  "forced-creature": [{
    description: "target creature",
    min_targets: 1,
    max_targets: 1,
    legal_targets: [{ kind: "object", object: 20, name: "Goblin Piker" }],
  }],
  // Optional targets are a choice even when only one is legal.
  "optional-opponent": [{
    description: "up to one target player",
    min_targets: 0,
    max_targets: 1,
    legal_targets: [{ kind: "player", player: 1, name: "Bob" }],
  }],
};

// A dies trigger: by the time it is on the stack its source has already moved
// to the graveyard and been given a NEW object id. inspect_object_id still
// points at the battlefield id it had, which no longer resolves.
const orphanSource = params.get("orphan") === "1";

const stackEntry = {
  id: 30,
  inspect_object_id: orphanSource ? 999 : 10,
  stable_id: 7,
  source_stable_id: 7,
  name: "Lava Spike",
  controller: 0,
  ability_kind: null,
  oracle_text: "Lava Spike deals 3 damage to target player.",
};

const decision = {
  kind: "targets",
  player: 0,
  source_id: 10,
  description: "Choose targets for Lava Spike",
  requirements: REQUIREMENTS[scenario],
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
  // The real field names: getVisibleStackObjects reads these, and a resolving
  // ability is exactly the case this fixture is about.
  stack_objects: [stackEntry],
  resolving_stack_object: params.get("resolving") === "1" ? stackEntry : null,
  players: [0, 1, 2].map((id) => ({
    id,
    index: id,
    name: ["Alice", "Bob", "Carol"][id],
    life: 20,
    graveyard_cards: id === 0 && orphanSource
      ? [{
        id: 41,
        stable_id: 7,
        name: "Sephiroth, Fabled SOLDIER",
        controller: 0,
        owner: 0,
        type_line: "Legendary Creature — Human Soldier",
        power: 4,
        toughness: 4,
        oracle_text: "Whenever another creature dies, target opponent loses 1 life and you gain 1 life.",
        semantic_score: 1,
      }]
      : [],
    battlefield: id === 0 && !orphanSource
      ? [{
        id: 10,
        stable_id: 7,
        name: "Sephiroth, Fabled SOLDIER",
        controller: 0,
        owner: 0,
        lane: "creatures",
        type_line: "Legendary Creature — Human Soldier",
        power: 4,
        toughness: 4,
        oracle_text: "Whenever another creature dies, target opponent loses 1 life and you gain 1 life.",
        semantic_score: 1,
      }]
      : id === 1
      ? [{
        id: 20,
        stable_id: 20,
        name: "Goblin Piker",
        controller: 1,
        owner: 1,
        lane: "creatures",
        type_line: "Creature — Goblin Warrior",
        power: 2,
        toughness: 1,
        oracle_text: "",
        semantic_score: 1,
      }]
      : [],
    hand_cards: [],
    exile_cards: [],
    command_cards: [],
    mana_pool: {},
  })),
};

window.__dispatched = null;
window.__cancelled = 0;

function HoverProbe() {
  const { hoveredObjectId } = useHover();
  useEffect(() => {
    window.__hoveredObjectId = hoveredObjectId;
  }, [hoveredObjectId]);
  return null;
}

export function Fixture() {
  return (
    <I18nProvider>
      <GameContext.Provider value={{
        state,
        multiplayer: { mode: "idle" },
        matchClockStore: { subscribe: () => () => {}, getSnapshot: () => null },
        playerAccentOverrides: {},
        game: null,
        holdRule: "never",
        setHoldRule: () => {},
        cancelDecision: () => { window.__cancelled += 1; },
        dispatch: async (command) => { window.__dispatched = command; },
        dispatchInBackground: async () => {},
      }}>
        <HoverProvider>
          <DragProvider>
            <CombatArrowProvider>
              <TooltipProvider>
                <HoverProbe />
                <main style={{ height: "96vh", padding: 24 }}>
                  <div style={{ width: 260 }} data-stack-host data-stack-preview-anchor="true">
                    <StackCard entry={stackEntry} />
                  </div>
                  <TargetsDecision decision={decision} canAct />
                  {/* Mounted with the very props every Workspace dock uses. */}
                  <RightRail
                    pinnedObjectId={null}
                    selectedObjectId={null}
                    inline
                    allowHoverFallback={allowHoverFallback}
                  />
                  {/* The surface Workspace actually uses for hover previews. */}
                  <FloatingCardPreview pinnedObjectId={null} excludedObjectIds={[]} />
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
