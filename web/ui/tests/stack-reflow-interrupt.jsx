import React from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import StackTimelineRail from "../src/components/right-rail/StackTimelineRail";
import "../src/index.css";
const spell = {id: 1, name: "Counterspell", controller: 0, ability_kind: null, text: "Counter target spell."};
const trigger = {id: 2, name: "Ivy, Gleeful Spellthief", controller: 0, ability_kind: "Triggered", text: "Copy that spell."};
// Casting a spell and answering its trigger churns the stack faster than one
// reflow animation lasts, which is the sequence that used to strand entries.
const sequence = [[], [spell], [spell, trigger], [], [spell], [spell, trigger], [spell], [spell, trigger]];
const step = Number(new URLSearchParams(location.search).get("step") || 35);
const root = createRoot(document.getElementById("root"));
const show = (stack, resolving = null) => {
  const state = {perspective: 0, players: [{id: 0, name: "Alice"}], stack_objects: stack, resolving_stack_object: resolving, stack_size: stack.length, decision: {kind: "priority", player: 0, actions: []}};
  root.render(<I18nProvider><GameContext.Provider value={{state}}><HoverProvider>
    <div data-my-zone style={{position: "relative", margin: 30}}>
      <div className="my-zone-board-shell" style={{position: "relative", height: 520, width: 900}}>
        <div className="my-zone-stack-rail"><StackTimelineRail inlineFlow /></div>
      </div>
    </div>
  </HoverProvider></GameContext.Provider></I18nProvider>);
};
window.__showStack = show;
show([]);
if (!new URLSearchParams(location.search).has("manual")) sequence.forEach((stack, index) => setTimeout(() => show(stack), 80 + index * step));
