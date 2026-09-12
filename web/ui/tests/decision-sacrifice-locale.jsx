import React from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { ObjectSelectionProvider } from "../src/context/ObjectSelectionContext";
import SelectObjectsDecision from "../src/components/decisions/SelectObjectsDecision";
import "../src/index.css";

const params = new URLSearchParams(location.search);
localStorage.setItem("ironsmith.locale", params.get("locale") || "es");

// Paying Yawgmoth's sacrifice cost: an engine frame wrapping a cost quoted
// from the card, over candidate rows that name cards.
const YAWGMOTH_COMPILED = "Protection from Humans\nPay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.\n{B}{B}, Discard a card: Proliferate.";
const yawgmoth = {id: 8, name: "Yawgmoth, Thran Physician", controller: 0, oracle_text: YAWGMOTH_COMPILED};
const decision = {
  kind: "select_objects",
  player: 0,
  source_id: 8,
  source_name: "Yawgmoth, Thran Physician's ability",
  reason: "Sacrifice",
  description: "Choose a creature to sacrifice: Sacrifice another creature",
  min: 1,
  max: 1,
  candidates: [
    {id: 11, name: "Ornithopter", legal: true},
    {id: 12, name: "Myr Moonvessel", legal: true},
    {id: 13, name: "Llanowar Elves", legal: true},
  ],
};
const state = {perspective: 0, players: [{id: 0, name: "Alice", battlefield: [yawgmoth]}], stack_objects: [], decision};

createRoot(document.getElementById("root")).render(
  <I18nProvider><GameContext.Provider value={{state}}><ObjectSelectionProvider><HoverProvider>
    <div data-sacrifice-probe><SelectObjectsDecision decision={decision} canAct layout="panel" /></div>
  </HoverProvider></ObjectSelectionProvider></GameContext.Provider></I18nProvider>
);
