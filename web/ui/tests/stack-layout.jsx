import React from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import StackTimelineRail from "../src/components/right-rail/StackTimelineRail";
import "../src/index.css";
const stack = Array.from({length: 8}, (_, index) => ({id: index + 1, name: "Island", controller: 0, ability_kind: "Triggered", text: "Draw a card."}));
const state = {perspective: 0, players: [{id:0,name:"Alice"}], stack_objects: stack, stack_size: stack.length, decision: {kind:"priority",player:0,actions:[]}};
createRoot(document.getElementById("root")).render(<I18nProvider><GameContext.Provider value={{state}}><HoverProvider>
  <div data-my-zone style={{position:"relative",margin:30}}>
    <div className="my-zone-board-shell" style={{position:"relative",height:180,width:500}}>
      <div className="my-zone-stack-rail"><StackTimelineRail inlineFlow /></div>
    </div>
  </div>
</HoverProvider></GameContext.Provider></I18nProvider>);
