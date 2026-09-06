import React, { useState, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { DragProvider, useDragActions, useDragState } from "../src/context/DragContext";
import { HoverProvider } from "../src/context/HoverContext";
import FloatingCardPreview from "../src/components/right-rail/FloatingCardPreview";
import { I18nProvider } from "../src/i18n/I18nContext";
import ActionPopover from "../src/components/overlays/ActionPopover";
import PlayerZonePiles from "../src/components/board/PlayerZonePiles";
import "../src/index.css";

function Fixture() {
  const [actionsOpen, setActionsOpen] = useState(false);
  const drag = useDragState();
  const { startDrag, markCastIntent, setCastTargetPreview, updateDrag } = useDragActions();
  useEffect(() => {
    if (drag?.castIntent && !drag.castIntent.targetDecision) setCastTargetPreview(99, drag.castIntent.startedAt, {
      kind:"targets", player:0, requirements:[{legal_targets:[{kind:"object",object:20},{kind:"object",object:31}]}],
    });
  }, [drag, setCastTargetPreview]);
  useEffect(() => {
    const move = event => updateDrag(event.clientX, event.clientY);
    window.addEventListener("pointermove", move);
    return () => window.removeEventListener("pointermove", move);
  }, [updateDrag]);
  const [targeting, setTargeting] = useState(false);
  const [selected, setSelected] = useState(null);
  const cards = Array.from({length:20}, (_, i) => ({ id:20-i, name:i % 2 ? "Island" : "Plains" }));
  const player = {id:0, name:"Alice", graveyard_size:cards.length, graveyard_cards:cards, exile_cards:[{id:30,name:"Hidden card",face_down:true}, {id:31,name:"Swamp"}]};
  const state = {players:[player], perspective:0,decision:targeting?{kind:"targets",player:0}:{kind:"priority",player:0}};
  return <GameContext.Provider value={{state}}>
    {!actionsOpen && <FloatingCardPreview pinnedObjectId={targeting ? null : selected} />}
    <button onClick={() => setActionsOpen(true)}>Show cast choices</button>
    {actionsOpen && <div style={{position:"relative",zIndex:1,transform:"translateZ(0)"}}>
      <div className="floating-card-preview" style={{position:"fixed",inset:0}} />
      <ActionPopover anchorRect={{left:180,top:300,bottom:300,width:0}} variant="game"
        actions={[{index:0,label:"Cast",kind:"cast_spell"},{index:1,label:"Cast without paying",kind:"cast_spell"}]}
        collapseEquivalentActions={false} onClose={() => setActionsOpen(false)} onAction={() => {setSelected("cast");setActionsOpen(false);}} />
    </div>}
    <button onClick={() => { startDrag(99,"Spell",[],null,900,700); markCastIntent({x:900,y:700}); }}>Start shortcut targeting</button>
    <button onClick={()=>setTargeting(!targeting)}>Toggle targeting</button><output>{selected ?? "none"}</output>
    <div style={{margin:20,width:"calc(100% - 40px)",height:400}}>
      <header style={{height:44}}>20 Alice</header>
      <div className="has-zone-piles" style={{height:350,background:"#141414"}}>
        <PlayerZonePiles player={player} legalTargetObjectIds={new Set([20])} onCardClick={(_,card)=>setSelected(card.id)} />
        <div className="battlefield-row" style={{position:"relative",height:330,paddingTop:50}}>
          <div className="battlefield-row-card" style={{marginLeft:160,width:72,height:100,background:"#776644"}}>Creature</div>
        </div>
      </div>
    </div>
  </GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><HoverProvider><DragProvider><Fixture /></DragProvider></HoverProvider></I18nProvider>);
