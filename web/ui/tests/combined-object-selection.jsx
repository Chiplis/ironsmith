import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { ObjectSelectionProvider } from "../src/context/ObjectSelectionContext";
import SelectObjectsDecision from "../src/components/decisions/SelectObjectsDecision";
import "../src/index.css";
const decision = { kind:"select_objects", player:0, description:"Discard one creature card or two cards", min:1,max:2,
  candidates:[{id:10,name:"Creature",legal:true},{id:20,name:"Land",legal:true},{id:30,name:"Instant",legal:true}],
  selection_alternatives:[{min:1,max:1,candidate_indices:[0]},{min:2,max:2,candidate_indices:[0,1,2]}] };
function Fixture() {
  const [command,setCommand]=useState(null);
  return <GameContext.Provider value={{state:{perspective:0,decision,players:[]},dispatch:setCommand}}>
    <HoverProvider><ObjectSelectionProvider><div style={{width:600,padding:30}}>
      <SelectObjectsDecision decision={decision} canAct layout="strip" />
      <output data-command>{JSON.stringify(command)}</output>
    </div></ObjectSelectionProvider></HoverProvider>
  </GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
