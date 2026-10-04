import React, { useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { ManaPaymentEditorProvider } from "../src/context/ManaPaymentEditorContext";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import { I18nProvider } from "../src/i18n/I18nContext";
import HandZone from "../src/components/board/HandZone";
import BattlefieldRow from "../src/components/board/BattlefieldRow";
import ManaPaymentDecision from "../src/components/decisions/ManaPaymentDecision";
import "../src/index.css";
const base = { transaction_id:"spell", plan_id:"initial", request_hash:"request", source_name:"Editable spell", can_confirm:true, planning_complete:false, pips:[["1"],["B","U","P"]],
  cost_context:["Flashback", "Kicker ×1"], pool_before:{}, pool_after_activations:{black:1,red:1},pool_after_payment:{},
  planned_sources:[{source_id:"1",source_name:"Mountain",payment_kind:"mana_ability",ability_index:0,expected_mana:{red:1},undo_safe:true},{source_id:"2",source_name:"Prism",payment_kind:"mana_ability",ability_index:0,color_restriction:["black"],expected_mana:{black:1},undo_safe:true}],
  available_sources:[{source_id:"1",source_name:"Mountain",payment_kinds:["manaability"]},{source_id:"2",source_name:"Prism",payment_kinds:["manaability"]},{source_id:"3",source_name:"Helper",payment_kinds:["convoke"]}],
  activation_options:[{source_id:"1",source_name:"Mountain",ability_index:0,expected_mana:{red:1},label:"{T}: Add {R}."},{source_id:"2",source_name:"Prism",ability_index:0,color_restriction:["black"],expected_mana:{black:1},label:"{T}: Add any color."},{source_id:"2",source_name:"Prism",ability_index:0,color_restriction:["blue"],expected_mana:{blue:1},label:"{T}: Add any color."}],
  mana_abilities:[{source_id:"1",source_name:"Mountain",ability_index:0,label:"{T}: Add {R}."},{source_id:"2",source_name:"Prism",ability_index:0,label:"{T}: Add any color."},{source_id:"2",source_name:"Prism",ability_index:1,label:"Choose an artifact to sacrifice: Add {U}."}], life_options:[{pip_id:1,life:2}], reserved_sources:[{source_id:"4",source_name:"Reserved relic",reason:"Reserved for a tap cost"}] };
const cards=[{id:1,name:"Mountain",type_line:"Basic Land — Mountain",lane:"lands"},{id:2,name:"Prism",type_line:"Artifact",lane:"artifacts"}].map(c=>({...c,stable_id:c.id,controller:0,owner:0,oracle_text:"{T}: Add mana."}));
function Fixture() {
  const [payment,setPayment]=useState(base),[commands,setCommands]=useState([]),[stops,setStops]=useState(0),[starts,setStarts]=useState(0),[busy,setBusy]=useState(false),[delay,setDelay]=useState(120);
  const [held,setHeld]=useState(false);
  const [handCards,setHandCards]=useState([]);
  const releases=useRef([]);
  const state={perspective:0,snapshot_id:1,decision:{kind:"mana_payment",player:0,source_id:99,subject:payment?.source_name,plan_id:payment?.plan_id,request_hash:payment?.request_hash},mana_payment:payment,players:[{id:0,name:"Alice",battlefield:cards,hand_cards:handCards,can_view_hand:true}]};
  const dispatch=async command=>{
    setCommands(old=>[...old,command]);
    if(command.response.action==="activate") {
      const sourceId=command.response.source_id;
      setPayment(old=>({...old,planning_complete:true,plan_id:"after-activation",request_hash:"after-activation",
        pool_before:{blue:1}, required_activations:(old.required_activations || []).filter(value=>value.source_id!==sourceId),
        planned_sources:old.planned_sources.filter(value=>value.source_id!==sourceId)}));
      return;
    }
    if(command.response.action!=="replan")return;
    setBusy(true);
    if (held) await new Promise(resolve=>releases.current.push(resolve));
    else await new Promise(resolve=>setTimeout(resolve,delay));
    const preferences=command.response;
    setPayment(old=>old && ({...old,...preferences,request_hash:`hash:${JSON.stringify(preferences)}`,plan_id:`plan:${JSON.stringify(preferences)}`,planning_complete:true,
      planned_sources:old.planned_sources.filter(source=>!preferences.excluded_source_ids.includes(source.source_id)),can_confirm:!preferences.excluded_source_ids.includes("1")}));
    setBusy(false);
  };
  const cancelBackgroundDispatch=()=>{setStops(n=>n+1);setPayment(old=>({...old,planning_complete:true}));};
  return <GameContext.Provider value={{state,loading:busy,dispatch,dispatchInBackground:()=>setStarts(n=>n+1),cancelBackgroundDispatch,cancelDecision:async options=>{setCommands(old=>[...old,{type:"cancel_decision",options}]);setPayment(null);releases.current.splice(0).forEach(resolve=>resolve());}}}>
    <ManaPaymentEditorProvider state={state} dispatch={dispatch} cancelBackgroundDispatch={cancelBackgroundDispatch}>
    <HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
      <button onClick={()=>{
        setHandCards([{id:5,stable_id:5,name:"Hand mana source",card_types:["Creature"],controller:0,owner:0}]);
        const source={source_id:"5",source_name:"Hand mana source",ability_index:0,expected_mana:{red:1},payment_kind:"mana_ability",max_activations:1};
        setPayment({...base,planning_complete:true,activation_options:[...base.activation_options,source]});
      }}>Hand source plan</button>
      <button onClick={()=>setDelay(500)}>Slow engine</button>
      <button onClick={()=>{
        const wall={source_id:'10',source_name:'Wall of Roots',ability_index:0,expected_mana:{green:1},max_activations:1,repeatable:false,payment_kind:'mana_ability',undo_safe:true};
        const battery={source_id:'11',source_name:'Counter source',ability_index:0,expected_mana:{green:1},max_activations:2,repeatable:true,payment_kind:'mana_ability',undo_safe:false};
        setPayment({...base,transaction_id:'limited',planning_complete:true,pips:[['4']],life_options:[],mana_abilities:[],planned_sources:[wall,battery,battery,base.planned_sources[0]],activation_options:[wall,battery,base.activation_options[0]],cost_context:[]});
      }}>Limited source plan</button>
      <button onClick={()=>{
        const sources=Array.from({length:14},(_,i)=>({source_id:String(i+20),source_name:`Land ${i+1}`,ability_index:0,expected_mana:{green:1},max_activations:1,payment_kind:'mana_ability',undo_safe:true}));
        setPayment({...base,transaction_id:'long',planning_complete:true,pips:[['14']],life_options:[],mana_abilities:[],planned_sources:sources,activation_options:sources,cost_context:[]});
      }}>Long payment plan</button>
      <button onClick={()=>setPayment({...base,transaction_id:'warnings',planning_complete:true,life_to_pay:2,warnings:['UsesNonUndoSafeSource(1)','ProducesExcessMana'],planned_sources:base.planned_sources.map(source=>({...source,undo_safe:false}))})}>Warning plan</button>
      <button onClick={()=>setPayment({...base,transaction_id:'undo-only',planning_complete:true,warnings:['UsesNonUndoSafeSource(1)'],planned_sources:base.planned_sources.map(source=>({...source,undo_safe:false}))})}>Undo-only plan</button>
      <button onClick={()=>setHeld(true)}>Hold engine</button>
      <button onClick={()=>{setHeld(false);releases.current.splice(0).forEach(resolve=>resolve());}}>Release engine</button>
      <output hidden data-commands>{JSON.stringify(commands)}</output><output hidden data-payment>{JSON.stringify(payment)}</output><output hidden data-background>{JSON.stringify({starts,stops})}</output>
      <div style={{width:"min(650px,100%)",height:"calc(70dvh + 24px)",padding:12}}><ManaPaymentDecision canAct decision={state.decision}/></div>
      <div style={{width:"min(700px,100%)",height:200,marginTop:50}}><BattlefieldRow cards={cards}/></div>
    {handCards.length>0 && <div style={{position:"fixed",bottom:0,left:0,width:500,height:180}}><HandZone player={state.players[0]} isExpanded layout="fan"/></div>}
    </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider>
    </ManaPaymentEditorProvider>
  </GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Fixture/></I18nProvider>);
