import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import ManaPaymentDecision from "../src/components/decisions/ManaPaymentDecision";
import "../src/index.css";
function Fixture() {
  const [command, setCommand] = useState(null);
  const decision = {kind:"mana_payment", player:0, subject:"Delve spell"};
  const payment = {plan_id:"1",request_hash:"2",planning_complete:true,can_confirm:true,source_name:"Delve spell",
    planned_sources:[{source_id:"42",source_name:"Chosen graveyard card",payment_kind:"delve",undo_safe:false}],
    available_sources:[],allocations:[{printed_index:0,payment_kind:"delve",source_id:"42"}],mana_pips:["1"]};
  return <GameContext.Provider value={{state:{perspective:0,decision,mana_payment:payment,players:[]},dispatch:setCommand}}>
    <HoverProvider><div style={{width:650,padding:30}}><ManaPaymentDecision decision={decision} canAct />
      <output data-command>{JSON.stringify(command)}</output></div></HoverProvider>
  </GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Fixture /></I18nProvider>);
