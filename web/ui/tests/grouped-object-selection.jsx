import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { ObjectSelectionProvider } from "../src/context/ObjectSelectionContext";
import SelectObjectsDecision from "../src/components/decisions/SelectObjectsDecision";
import "../src/index.css";
const card=(id,owner=0)=>({id,stable_id:id,name:'Forest',oracle_text:'{T}: Add {G}.',owner});
const cards=[card(10),card(20),card(30),card(40,1)];
const decision={kind:'select_objects',player:0,description:'Choose two cards',min:2,max:2,
 candidates:cards.map(c=>({id:c.id,name:c.name,legal:true}))};
function Fixture() {
  const [command,setCommand]=useState(null);
  return <GameContext.Provider value={{state:{perspective:0,decision,players:[{id:0,name:"Alice",hand_cards:cards.slice(0,3)},{id:1,name:"Bob",hand_cards:cards.slice(3)}]},dispatch:setCommand}}>
    <HoverProvider><ObjectSelectionProvider><div className="battlefield-human-decision-panel" style={{width:340,padding:12}}>
      <SelectObjectsDecision decision={decision} canAct layout="strip" />
      <output data-command>{JSON.stringify(command)}</output>
    </div></ObjectSelectionProvider></HoverProvider>
  </GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
