import React from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { I18nProvider } from "../src/i18n/I18nContext";
import DecisionSummary from "../src/components/decisions/DecisionSummary";
import "../src/index.css";

const params = new URLSearchParams(location.search);
localStorage.setItem("ironsmith.locale", params.get("locale") || "es");

// The compiled text the engine carries for each card, and the prompts it
// phrases from that text.
const IVY_COMPILED = "Flying\nWhenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.";
const YAWGMOTH_COMPILED = "Protection from Humans\nPay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.\n{B}{B}, Discard a card: Proliferate.";
const ivy = {id: 7, name: "Ivy, Gleeful Spellthief", controller: 0, oracle_text: IVY_COMPILED, type_line: "Legendary Creature — Faerie Rogue"};
const yawgmoth = {id: 8, name: "Yawgmoth, Thran Physician", controller: 0, oracle_text: YAWGMOTH_COMPILED, type_line: "Legendary Creature — Phyrexian Human Cleric"};
const sourceId = Number(params.get("source") || 7);
const decision = {
  kind: "select_options",
  player: 0,
  source_id: sourceId,
  source_name: sourceId === 8 ? "Yawgmoth, Thran Physician's ability" : "Ivy, Gleeful Spellthief",
  description: params.get("description") || "Copy that spell",
  options: [{index: 1, description: "Yes"}, {index: 0, description: "No"}],
};
const state = {perspective: 0, players: [{id: 0, name: "Alice", battlefield: [ivy, yawgmoth]}], stack_objects: [], decision};

createRoot(document.getElementById("root")).render(
  <I18nProvider><GameContext.Provider value={{state}}>
    <div data-decision-summary-probe><DecisionSummary decision={decision} layout="panel" /></div>
  </GameContext.Provider></I18nProvider>
);
