import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider, useHoveredObjectId } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import TableCore from "../src/components/board/TableCore";
import FloatingCardPreview from "../src/components/right-rail/FloatingCardPreview";
import SelectOptionsDecision from "../src/components/decisions/SelectOptionsDecision";
import { hoveredObjectZoneViews } from "../src/lib/stack-targets";
import { optionForClickedObject } from "../src/lib/decision-object-meta";
import "../src/index.css";

const permanent = (id, name, controller) => ({
  id,
  stable_id: id,
  name,
  controller,
  owner: controller,
  lane: "creatures",
  type_line: "Creature — Human",
  power: 2,
  toughness: 2,
  oracle_text: "",
  semantic_score: 1,
});

const graveyardCard = {
  id: 40,
  stable_id: 40,
  name: "Buried Alive",
  controller: 0,
  owner: 0,
  type_line: "Sorcery",
  oracle_text: "Search your library for up to three creature cards.",
  card_types: ["Sorcery"],
  semantic_score: 1,
};

// "Choose a creature to sacrifice": each option is linked to a real object.
const decision = {
  kind: "select_options",
  player: 0,
  reason: "sacrifice",
  description: "Sacrifice a creature",
  min_choices: 1,
  max_choices: 1,
  options: [
    { index: 0, description: "Sacrifice Devoted Druid", object_id: 20 },
    { index: 1, description: "Return Buried Alive", object_id: 40 },
    { index: 2, description: "Decline" },
  ],
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
  stack_objects: [],
  players: [0, 1].map((id) => ({
    id,
    index: id,
    name: id ? "Bob" : "Alice",
    life: 20,
    battlefield: id === 0
      ? [permanent(10, "Yawgmoth, Thran Physician", 0), permanent(20, "Devoted Druid", 0)]
      : [],
    hand_cards: [],
    graveyard_cards: id === 0 ? [graveyardCard] : [],
    exile_cards: [],
    command_cards: [],
    mana_pool: {},
  })),
};

export function Fixture() {
  const [zoneViews, setZoneViews] = useState([]);
  const hoveredObjectId = useHoveredObjectId();
  // Exactly what Workspace does: a hovered object in a pile opens that pile.
  const temporary = useMemo(
    () => hoveredObjectZoneViews(state, hoveredObjectId, zoneViews),
    [hoveredObjectId, zoneViews]
  );
  const effectiveZoneViews = useMemo(
    () => Array.from(new Set([...zoneViews, ...temporary])),
    [temporary, zoneViews]
  );
  useEffect(() => {
    window.__zoneViews = effectiveZoneViews;
    window.__hovered = hoveredObjectId;
    window.__state = state;
  }, [effectiveZoneViews, hoveredObjectId]);

  return (
    <>
      <FloatingCardPreview pinnedObjectId={null} excludedObjectIds={[]} />
      <main style={{ height: "80vh" }}>
        <TableCore
          zoneViews={effectiveZoneViews}
          setZoneViews={setZoneViews}
          onInspect={(objectId) => {
            const option = optionForClickedObject(decision, objectId);
            if (option) {
              window.dispatchEvent(new CustomEvent("ironsmith:select-option-choice", {
                detail: { optionIndex: option.index },
              }));
            }
          }}
          middleTopbar={<div style={{ height: 60 }}><div className="topbar-main-decision-host" data-topbar-main-decision-host="true" /></div>}
        />
      </main>
      <div data-decision-host style={{ position: "fixed", bottom: 0, left: 0, width: 420, zIndex: 40000, background: "#0b1118" }}>
        <SelectOptionsDecision decision={decision} canAct />
      </div>
    </>
  );
}

createRoot(document.getElementById("root")).render(
  <I18nProvider>
    <GameContext.Provider value={{
      state,
      multiplayer: { mode: "idle" },
      matchClockStore: { subscribe: () => () => {}, getSnapshot: () => null },
      playerAccentOverrides: {},
      game: {
        objectDetails: async (id) => {
          const all = [
            ...state.players[0].battlefield,
            ...state.players[0].graveyard_cards,
          ];
          return all.find((c) => String(c.id) === String(id)) || null;
        },
      },
      holdRule: "never",
      setHoldRule: () => {},
      cancelDecision: () => {},
      dispatch: async (command, label) => { window.__dispatched = { command, label }; },
      dispatchInBackground: async () => {},
    }}>
      <HoverProvider>
        <DragProvider>
          <CombatArrowProvider>
            <TooltipProvider>
              <Fixture />
            </TooltipProvider>
          </CombatArrowProvider>
        </DragProvider>
      </HoverProvider>
    </GameContext.Provider>
  </I18nProvider>
);
