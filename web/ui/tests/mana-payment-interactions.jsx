import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider, useHover } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import FloatingCardPreview from "../src/components/right-rail/FloatingCardPreview";
import BattlefieldRow from "../src/components/board/BattlefieldRow";
import "../src/index.css";
const cards = [
  {id:1,member_ids:[1,4],name:"Mountain",type_line:"Basic Land — Mountain",lane:"lands",oracle_text:"{T}: Add {R}."},
  {id:2,name:"Mana Prism",type_line:"Artifact",lane:"artifacts",oracle_text:"{T}: Add {C}.\n{1}, {T}: Add one mana of any color."},
  {id:3,name:"Ornithopter",type_line:"Artifact Creature",lane:"creatures",power:0,toughness:2},
].map(card=>({...card,stable_id:card.id,controller:0,owner:0}));
const abilities = [
  {source_id:"4",ability_index:0,source_name:"Mountain",label:"{T}: Add {R}."},
  {source_id:"1",ability_index:0,source_name:"Mountain",label:"{T}: Add {R}."},
  {source_id:"2",ability_index:0,source_name:"Mana Prism",label:"{T}: Add {C}."},
  {source_id:"2",ability_index:1,source_name:"Mana Prism",label:"{1}, {T}: Add one mana of any color."},
];
function HoverStatus() { const {hoveredObjectId} = useHover(); return <output data-hover>{hoveredObjectId ?? "none"}</output>; }
function Fixture() {
  const [kind,setKind] = useState("mana_payment");
  const [payer,setPayer] = useState(0);
  const [tapped,setTapped] = useState(false);
  const [command,setCommand] = useState(null);
  const [inspected,setInspected] = useState(null);
  const [loading,setLoading] = useState(false);
  const [filterPrism,setFilterPrism] = useState(false);
  const state = {snapshot_id:tapped?2:1,perspective:0,players:[{id:0,battlefield:cards}],decision:{kind,player:payer,source_id:99},mana_payment:kind==="mana_payment"?{request_hash:"payment",mana_abilities:abilities.filter(a=>(!tapped||a.source_id!=="1") && (!filterPrism||a.source_id!=="2"||a.ability_index===1))}:null};
  return <GameContext.Provider value={{state,loading,dispatch:command=>{setCommand(command);if(command.response.source_id==="1")setTapped(true);else setKind("select_objects");}}}>
    <HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
      <FloatingCardPreview pinnedObjectId={3} />
      <button onClick={()=>setKind("mana_payment")}>Resume payment</button>
      <button onClick={()=>setPayer(payer===0?1:0)}>Switch payer</button>
      <button onClick={()=>setLoading(!loading)}>Toggle busy</button>
      <button onClick={()=>setFilterPrism(true)}>Filter Prism to one ability</button>
      <HoverStatus/><output data-inspected>{inspected ?? "none"}</output><output data-command>{JSON.stringify(command)}</output>
      <output data-decision>{kind}</output>
      <div style={{margin:100,width:700,height:280}}><BattlefieldRow cards={cards.map(c=>({...c,tapped:c.id===1&&tapped}))} onInspect={setInspected}/></div>
    </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider>
  </GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Fixture/></I18nProvider>);
