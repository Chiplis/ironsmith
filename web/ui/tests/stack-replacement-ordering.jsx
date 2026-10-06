import React, { useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import FloatingCardPreview from "../src/components/right-rail/FloatingCardPreview";
import StackTimelineRail from "../src/components/right-rail/StackTimelineRail";
import DecisionPopupLayer from "../src/components/overlays/DecisionPopupLayer";
import { buildEffectOrderingKey, normalizeEffectOrderingOrder } from "../src/lib/effect-ordering";
import "../src/index.css";

const params = new URLSearchParams(location.search);
const mobile = params.has("mobile");
const spectator = params.has("spectator");
const agent = (id, controller) => ({
  id, stable_id: id, name: "Opposition Agent", type_line: "Creature — Human Rogue",
  oracle_text: "You control your opponents while they're searching their libraries.",
  compiled_text: ["You control your opponents while they're searching their libraries."],
  zone: "Battlefield", controller, owner: controller, power: 3, toughness: 2,
});
const agents = params.has("optional")
  ? [{ ...agent(40, 0), name: "Golgari Thug", oracle_text: "Dredge 4", compiled_text: ["Dredge 4"], zone: "Graveyard" }]
  : [agent(40, 1), agent(41, 2)];
const search = { id: 60, stable_id: 60, name: "Demonic Tutor", type_line: "Sorcery", zone: "Stack", controller: 0, owner: 0, oracle_text: "Search your library for a card.", compiled_text: ["Search your library for a card."] };
const cards = Object.fromEntries([...agents, search].map(card => [card.id, card]));
const decision = {
  kind: "select_options", player: 0, min: 1, max: 1,
  description: "Choose which replacement effect to apply",
  options: params.has("optional") ? [
    { index: 9, legal: true, object_id: 40, description: "Do not apply Golgari Thug" },
    { index: 3, legal: true, object_id: 40, description: "Golgari Thug" },
  ] : agents.map((card, index) => ({
    index, legal: true, related_object_ids: [card.id],
    description: `Opposition Agent\nExile The Underworld Cookbook; ${index === 0 ? "Bob" : "Carol"} may play that card for as long as it remains exiled.`,
  })),
};
window.__commands = [];
window.__inspections = [];

function Fixture() {
  const [order, setOrder] = useState([0, 1]);
  const state = useMemo(() => ({
    perspective: spectator ? 1 : 0, active_player: 0, priority_player: 0,
    phase: "NextMain", step: null,
    players: ["Alice", "Bob", "Carol"].map((name, id) => ({
      id, name, battlefield: agents.filter(card => card.controller === id),
      hand_cards: [], graveyard_cards: [], exile_cards: [], mana_pool: {},
    })),
    stack_objects: [{ ...search, inspect_object_id: 60, targets: [] }], stack_size: 1,
    decision,
  }), []);
  const value = useMemo(() => ({
    state, game: { objectDetails: async id => cards[Number(id)] || null },
    dispatch: command => window.__commands.push(command),
    multiplayer: { mode: "idle" }, holdRule: "never", setHoldRule: () => {}, playerAccentOverrides: {},
    effectOrderingState: { key: buildEffectOrderingKey(decision), order },
    moveEffectOrderingItem: (position, direction) => setOrder(current => {
      const next = normalizeEffectOrderingOrder(current, decision);
      const target = position + direction;
      if (target < 0 || target >= next.length) return next;
      [next[position], next[target]] = [next[target], next[position]];
      return next;
    }),
  }), [order, state]);
  return <GameContext.Provider value={value}><HoverProvider><DragProvider><TooltipProvider>
    <div className="topbar-main-decision-host" data-topbar-main-decision-host="true" style={{ position: "relative", margin: 20, width: 180, height: 60 }}>
      <DecisionPopupLayer priorityInline mobileBattle={mobile} mobileBattleDockInline />
    </div>
    <div style={{ position: "relative", width: params.has("narrow") ? 250 : 380, height: 500, margin: 20 }}>
      <StackTimelineRail inlineFlow onInspectObject={id => window.__inspections.push(id)} />
    </div>
    <FloatingCardPreview />
  </TooltipProvider></DragProvider></HoverProvider></GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
