import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { I18nProvider } from "../src/i18n/I18nContext";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider, useDragActions, useDragSession } from "../src/context/DragContext";
import StackCard from "../src/components/cards/StackCard";
import "../src/index.css";

// A spell on the stack is drawn as 272 and targeted as 136; the ability is
// drawn as 45 and inspects to the permanent 134 that produced it.
const bolt = { id: 272, inspect_object_id: 136, stable_id: 133, controller: 1, name: "Lightning Bolt", mana_cost: "{R}", targets: [] };
const fanatic = { id: 45, inspect_object_id: 134, stable_id: 132, controller: 1, name: "Mogg Fanatic", ability_kind: "Activated", ability_text: "It deals 1 damage to any target.", targets: [] };
// The permanent 134 and a permanent that collides with the spell's drawn id
// are legal too: neither may light a stack tile up.
const LEGAL = [
  { kind: "object", object: 136, name: "Lightning Bolt" },
  { kind: "object", object: 134, name: "Mogg Fanatic" },
  { kind: "object", object: 272, name: "Collision" },
];
const TARGETS = { kind: "targets", player: 0, requirements: [{ description: "target spell", min_targets: 1, max_targets: 1, legal_targets: LEGAL }] };
const PRIORITY = { kind: "priority", player: 0, actions: [] };
const CAST_ACTIONS = [
  { kind: "cast_spell", index: 1, drag_requires_targets: true, drag_requires_modes: false },
  { kind: "cast_spell", index: 2, drag_requires_targets: true, drag_requires_modes: false },
];

function Controls({ setMode }) {
  const { startDrag, markCastIntent, setCastTargetPreview, updateDrag, endDrag } = useDragActions();
  const drag = useDragSession();
  useEffect(() => {
    const onMove = (event) => updateDrag(event.clientX, event.clientY);
    document.addEventListener("pointermove", onMove);
    return () => document.removeEventListener("pointermove", onMove);
  }, [updateDrag]);
  return (
    <div style={{ display: "flex", gap: 8, padding: 8 }}>
      <button onClick={() => setMode("priority")}>Priority</button>
      <button onClick={() => setMode("live")}>Live targets</button>
      <button onClick={() => {
        startDrag(900, "Counterspell", CAST_ACTIONS, "instant", 600, 500, null,
          { id: 900, name: "Counterspell" }, { left: 0, top: 0, right: 0, bottom: 0 }, null);
        markCastIntent({ x: 600, y: 500 });
      }}>Start gesture</button>
      <button onClick={() => {
        if (drag?.castIntent) setCastTargetPreview(drag.objectId, drag.castIntent.startedAt, TARGETS);
      }}>Preview targets</button>
      <button onClick={() => endDrag()}>End gesture</button>
    </div>
  );
}

function Fixture() {
  const [mode, setMode] = useState("live");
  const state = useMemo(() => ({
    perspective: 0,
    players: [
      { id: 0, name: "Alice", battlefield: [] },
      { id: 1, name: "Bob", battlefield: [{ id: 134, name: "Mogg Fanatic", controller: 1 }, { id: 272, name: "Collision", controller: 1 }] },
    ],
    stack_objects: [bolt, fanatic],
    decision: mode === "live" ? TARGETS : PRIORITY,
  }), [mode]);
  return (
    <GameContext.Provider value={{ state, game: { objectDetails: async () => null }, dispatch: () => {} }}>
      <HoverProvider>
        <DragProvider>
          <Controls setMode={setMode} />
          <div data-stack-preview-anchor="true" style={{ width: 240, marginTop: 40, marginLeft: 20, display: "grid", gap: 6 }}>
            {[bolt, fanatic].map((entry) => (
              <StackCard
                key={entry.id}
                entry={entry}
                onClick={(id, meta) => {
                  window.__stackClicks = window.__stackClicks || [];
                  window.__stackClicks.push({ id, stackId: meta.stackEntry.id });
                }}
              />
            ))}
          </div>
        </DragProvider>
      </HoverProvider>
    </GameContext.Provider>
  );
}

createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
