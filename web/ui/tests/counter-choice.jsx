import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { I18nProvider } from "../src/i18n/I18nContext";
import DecisionRouter from "../src/components/decisions/DecisionRouter";
import "../src/index.css";
window.__commands=[];
const initial={kind:"select_counters",player:0,description:"Choose counters to remove",min_total:"0",max_total:"8589934590",options:[{index:0,description:"Charge counters",max_count:4294967295,legal:true},{index:1,description:"+1/+1 counters",max_count:4294967295,legal:true}]};
function Fixture(){
  const [decision,setDecision]=useState(initial);window.__setDecision=(patch)=>setDecision({...initial,...patch});
  const canAct=!new URLSearchParams(location.search).has("spectator");
  const state={decision,perspective:0,players:[],stack_objects:[],stack_size:0};
  return <I18nProvider><GameContext.Provider value={{state,dispatch:command=>window.__commands.push(command)}}>
    <div style={{height:450,maxWidth:600,padding:20}}><DecisionRouter decision={decision} canAct={canAct}/></div>
  </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById("root")).render(<Fixture/>);
