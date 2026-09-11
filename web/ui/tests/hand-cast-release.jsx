import { useCallback, useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider, useDragState } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { useCombatArrows } from "../src/context/useCombatArrows";
import { ObjectSelectionProvider } from "../src/context/ObjectSelectionContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import Workspace from "../src/components/layout/Workspace";
import "../src/index.css";

const unsummon = {
  id: 5,
  stable_id: 5,
  name: "Unsummon",
  type_line: "Instant",
  card_types: ["instant"],
  mana_cost: "{U}",
  oracle_text: "Return target creature to its owner's hand.",
  semantic_score: 1,
};
const bears = {
  id: 20,
  stable_id: 20,
  name: "Grizzly Bears",
  controller: 1,
  owner: 1,
  lane: "creatures",
  type_line: "Creature — Bear",
  card_types: ["creature"],
  power: 2,
  toughness: 2,
  oracle_text: "",
  semantic_score: 1,
};

const castAction = {
  index: 0,
  object_id: 5,
  kind: "cast_spell",
  label: "Cast Unsummon",
  drag_requires_targets: true,
  drag_requires_modes: false,
  action_ref: { kind: "cast_spell", spell_id: 5 },
};

// With something like Omniscience out, the same spell has more than one way
// to be cast, which keeps the gesture provisional until it is released.
const freeCastAction = {
  ...castAction,
  index: 1,
  label: "Cast Unsummon without paying its mana cost",
  action_ref: { kind: "cast_spell", spell_id: 5, casting_method: { kind: "free" } },
};
const castActions = new URLSearchParams(window.location.search).has("two")
  ? [castAction, freeCastAction]
  : [castAction];

const priorityDecision = { kind: "priority", player: 0, actions: castActions };
const targetsDecision = {
  kind: "targets",
  player: 0,
  source_id: 5,
  description: "Choose targets for Unsummon",
  requirements: [{
    description: "target creature",
    min_targets: 1,
    max_targets: 1,
    legal_targets: [{ kind: "object", object: 20, name: "Grizzly Bears" }],
  }],
};

window.__cancelled = 0;
window.__arrow = null;
window.__drag = null;
window.__dispatched = [];

/** The targeting arrow lives in context; the DOM only shows it if it can find
 *  the card it comes from, so the probe reads the state itself. */
export function ArrowProbe() {
  const { dragArrow } = useCombatArrows();
  useEffect(() => {
    window.__arrow = dragArrow ? { ...dragArrow } : null;
  }, [dragArrow]);
  return null;
}

/** The gesture's own state, which the arrow follows while it is held. */
export function DragProbe() {
  const drag = useDragState();
  useEffect(() => {
    window.__drag = drag
      ? { objectId: drag.objectId, held: Boolean(drag.held), castIntent: Boolean(drag.castIntent), x: drag.currentX, y: drag.currentY }
      : null;
  }, [drag]);
  return null;
}

export function Fixture() {
  // The engine's part of the exchange: casting Unsummon asks for a target.
  const [decision, setDecision] = useState(priorityDecision);

  const state = useMemo(() => ({
    perspective: 0,
    priority_player: 0,
    active_player: 0,
    snapshot_id: 1,
    phase: "Main",
    step: "Main",
    cancelable: true,
    decision,
    // The cast sits on the stack while it asks for a target, which is what
    // the targeting arrow anchors itself to.
    stack_objects: decision.kind === "targets"
      ? [{
        id: 90,
        stable_id: 90,
        name: "Unsummon",
        controller: 0,
        source_stable_id: 5,
        inspect_object_id: 90,
        ability_kind: "Spell",
        source_ability_text: "Return target creature to its owner's hand.",
        targets: [],
      }]
      : [],
    players: [0, 1].map((id) => ({
      id,
      index: id,
      name: id ? "Bob" : "Alice",
      life: 20,
      can_view_hand: id === 0,
      hand_cards: id === 0 && decision.kind === "priority" ? [unsummon] : [],
      hand_size: id === 0 && decision.kind === "priority" ? 1 : 0,
      battlefield: id === 1 ? [bears] : [],
      graveyard_cards: [],
      exile_cards: [],
      command_cards: [],
      mana_pool: {},
    })),
  }), [decision]);

  const dispatch = useCallback(async (command) => {
    window.__dispatched.push(command);
    if (command?.type === "priority_action") setDecision(targetsDecision);
  }, []);

  const value = useMemo(() => ({
    state,
    // The provisional gesture asks the engine what it could target without
    // casting anything yet.
    game: { previewCastTargets: async () => targetsDecision },
    multiplayer: { mode: "idle" },
    playerAccentOverrides: {},
    matchClockStore: { subscribe: () => () => {}, getSnapshot: () => null },
    holdRule: "never",
    setHoldRule: () => {},
    dispatch,
    dispatchInBackground: async () => {},
    cancelDecision: () => { window.__cancelled += 1; setDecision(priorityDecision); },
    setStatus: () => {},
  }), [dispatch, state]);

  return (
    <I18nProvider>
      <GameContext.Provider value={value}>
        <HoverProvider>
          <DragProvider>
            <CombatArrowProvider>
              <ObjectSelectionProvider>
                <TooltipProvider>
                  <ArrowProbe />
                  <DragProbe />
                  <div data-cast-release-case style={{ position: "fixed", inset: 0 }}>
                    <Workspace zoneViews={[]} setZoneViews={() => {}} />
                  </div>
                </TooltipProvider>
              </ObjectSelectionProvider>
            </CombatArrowProvider>
          </DragProvider>
        </HoverProvider>
      </GameContext.Provider>
    </I18nProvider>
  );
}

createRoot(document.getElementById("root")).render(<Fixture />);
