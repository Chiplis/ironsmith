import React, { useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import FloatingCardPreview from "../src/components/right-rail/FloatingCardPreview";
import StackTimelineRail from "../src/components/right-rail/StackTimelineRail";
import PlayerZonePiles from "../src/components/board/PlayerZonePiles";
import { buildTriggerOrderingKey, normalizeTriggerOrderingOrder } from "../src/lib/trigger-ordering";
import "../src/index.css";

// Two Blood Artist triggers and a Zulaport Cutthroat trigger wait to be
// ordered while a Lightning Bolt and a live Cutthroat trigger already sit on
// the stack. Each pending option names its source object the way the engine
// does, through related_object_ids.
const bloodArtist = {
  id: 40, stable_id: 40, name: "Blood Artist", type_line: "Creature — Vampire", power: "0", toughness: "1",
  oracle_text: "Whenever Blood Artist or another creature dies, target player loses 1 life and you gain 1 life.",
  compiled_text: ["Whenever Blood Artist or another creature dies, target player loses 1 life and you gain 1 life."],
  zone: "Graveyard", controller: 0, owner: 0,
};
const cutthroat = {
  id: 41, stable_id: 41, name: "Zulaport Cutthroat", type_line: "Creature — Human Rogue Ally", power: "1", toughness: "1",
  oracle_text: "Whenever Zulaport Cutthroat or another creature you control dies, each opponent loses 1 life and you gain 1 life.",
  compiled_text: ["Whenever Zulaport Cutthroat or another creature you control dies, each opponent loses 1 life and you gain 1 life."],
  zone: "Battlefield", controller: 0, owner: 0,
};
const bolt = {
  id: 60, stable_id: 60, name: "Lightning Bolt", type_line: "Instant", mana_cost: "{R}",
  oracle_text: "Lightning Bolt deals 3 damage to any target.", compiled_text: ["Lightning Bolt deals 3 damage to any target."],
  zone: "Stack", controller: 1, owner: 1,
};
const cards = { 40: bloodArtist, 41: cutthroat, 60: bolt };
const artistText = "Whenever Blood Artist or another creature dies, target player loses 1 life and you gain 1 life.";
const cutthroatText = "Whenever Zulaport Cutthroat or another creature you control dies, each opponent loses 1 life and you gain 1 life.";
const decision = {
  kind: "select_options", player: 0, reason: "Order triggers",
  description: "Order triggered abilities. The leftmost item becomes the top of your stack.",
  min: 3, max: 3,
  options: [
    { index: 0, description: `Blood Artist\n${artistText}\nTrigger 1`, legal: true, related_object_ids: [40] },
    { index: 1, description: `Blood Artist\n${artistText}\nTrigger 2`, legal: true, related_object_ids: [40] },
    { index: 2, description: `Zulaport Cutthroat\n${cutthroatText}`, legal: true, related_object_ids: [41] },
  ],
};
const stack = [
  { id: 121, inspect_object_id: 60, stable_id: 60, name: "Lightning Bolt", controller: 1, mana_cost: "{R}", effect_text: "Lightning Bolt deals 3 damage to any target.", targets: [] },
  { id: 83, inspect_object_id: 41, source_stable_id: 41, name: "Zulaport Cutthroat", controller: 0, ability_kind: "Triggered", source_ability_text: cutthroatText, ability_text: cutthroatText, power: 1, toughness: 1, targets: [] },
];
const game = { objectDetails: async (id) => cards[Number(id)] || null };
window.__inspections = [];

function Fixture() {
  const key = buildTriggerOrderingKey(decision);
  const [order, setOrder] = useState([0, 1, 2]);
  const state = useMemo(() => ({
    perspective: 0,
    players: [
      { id: 0, name: "Alice", battlefield: [cutthroat], hand_cards: [], graveyard_size: 2, graveyard_cards: [bloodArtist, { id: 42, stable_id: 42, name: "Swamp", zone: "Graveyard", controller: 0, owner: 0 }], exile_cards: [] },
      { id: 1, name: "Bob", battlefield: [], hand_cards: [], graveyard_cards: [], exile_cards: [] },
    ],
    stack_objects: stack,
    stack_size: stack.length,
    decision,
  }), []);
  const value = useMemo(() => ({
    state,
    game,
    dispatch: () => {},
    triggerOrderingState: { key, order },
    moveTriggerOrderingItem: (position, direction) => setOrder((current) => {
      const next = normalizeTriggerOrderingOrder(current, decision);
      const target = position + direction;
      if (target < 0 || target >= next.length) return next;
      [next[position], next[target]] = [next[target], next[position]];
      return next;
    }),
  }), [key, order, state]);
  return (
    <GameContext.Provider value={value}><HoverProvider><DragProvider>
      <div data-my-zone style={{ position: "relative", margin: 20 }}>
        <div className="my-zone-board-shell has-zone-piles" style={{ position: "relative", height: 560, width: 720 }}>
          <PlayerZonePiles player={state.players[0]} legalTargetObjectIds={new Set()} onCardClick={() => {}} />
          <div className="my-zone-stack-rail">
            <StackTimelineRail
              inlineFlow
              onInspectObject={(id, meta) => { window.__inspections.push({ id: String(id), entryId: String(meta?.stackEntry?.id) }); }}
            />
          </div>
        </div>
      </div>
      <FloatingCardPreview />
    </DragProvider></HoverProvider></GameContext.Provider>
  );
}

createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
