import React, { useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { I18nProvider } from "../src/i18n/I18nContext";
import { HoverProvider } from "../src/context/HoverContext";
import { DragProvider } from "../src/context/DragContext";
import { CombatArrowProvider } from "../src/context/CombatArrowContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import { Button } from "../src/components/ui/button";
import { Input } from "../src/components/ui/input";
import { Checkbox } from "../src/components/ui/checkbox";
import { Slider } from "../src/components/ui/slider";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "../src/components/ui/tabs";
import DecisionRouter from "../src/components/decisions/DecisionRouter";
import OpenDecklistModal from "../src/components/board/OpenDecklistModal";
import "../src/index.css";
import "./ui-audit.css";

const cards = [
  {id: 10, name: "Llanowar Elves", controller: 0, owner: 0, type_line: "Creature — Elf Druid", power: 1, toughness: 1},
  {id: 11, name: "Yawgmoth, Thran Physician", controller: 0, owner: 0, type_line: "Legendary Creature — Human Cleric", power: 2, toughness: 4},
];
const fixtures = {
  number: {kind: "number", description: "Choose a number from 0 to 10", min: 0, max: 10},
  text_input: {kind: "text_input", description: "Name a card", placeholder: "Enter a card name"},
  select_options: {kind: "select_options", description: "Choose up to two modes", min: 1, max: 2, options: [
    {index: 0, description: "Draw two cards.", legal: true},
    {index: 1, description: "Return target creature card from your graveyard to your hand, then gain life equal to its mana value.", legal: true},
    {index: 2, description: "Destroy target artifact.", legal: false},
  ]},
  select_objects: {kind: "select_objects", description: "Choose a creature to sacrifice", min: 1, max: 1, candidates: cards.map(card => ({...card, legal: true}))},
  targets: {kind: "targets", description: "Choose a target for Lightning Bolt", requirements: [{description: "Any target", min_targets: 1, max_targets: 1, legal_targets: [
    {kind: "player", player: 1, name: "Bob"}, {kind: "object", object: 11, name: "Yawgmoth, Thran Physician"},
  ]}]},
  attackers: {kind: "attackers", description: "Declare attackers", attacker_options: [{creature: 10, name: "Llanowar Elves", valid_targets: [{kind: "player", player: 1}]}]},
  blockers: {kind: "blockers", description: "Declare blockers", blocker_options: [{attacker: 20, attacker_name: "Grizzly Bears", valid_blockers: cards, min_blockers: 1}]},
  mana_payment: {kind: "mana_payment", description: "Pay {1}{G}"},
};

export default function Audit() {
  const [kind, setKind] = useState("number");
  const [layout, setLayout] = useState("panel");
  const [canAct, setCanAct] = useState(true);
  const [lastAction, setLastAction] = useState("No action submitted");
  const [deckOpen, setDeckOpen] = useState(false);
  const decision = useMemo(() => ({...fixtures[kind], player: 0}), [kind]);
  const context = useMemo(() => ({
    state: {players: [{id: 0, index: 0, name: "Alice", life: 20, battlefield: cards}, {id: 1, index: 1, name: "Bob", life: 20, battlefield: []}], perspective: 0, priority_player: 0, active_player: 0, decision, stack: [], mana_payment: kind === "mana_payment" ? {
      source_name: "Grizzly Bears", planning_complete: true, request_hash: "fixture", plan_id: "fixture",
      pips: [["1"], ["G"]], pool_before: {}, pool_after_activations: {green: 2}, pool_after_payment: {},
      planned_sources: [{source_id: "10", source_name: "Llanowar Elves", expected_mana: {green: 1}, undo_safe: true}, {source_id: "12", source_name: "Forest", expected_mana: {green: 1}, undo_safe: true}],
      available_sources: [], allocations: [], warnings: [], life_to_pay: 0,
    } : null},
    multiplayer: {mode: "idle", players: []},
    playerAccentOverrides: {},
    dispatch: (command) => {setLastAction(JSON.stringify(command)); return Promise.resolve();},
    dispatchInBackground: () => Promise.resolve(),
    game: {isKnownCardName: async (name) => ["island", "lightning bolt"].includes(name.toLowerCase())},
  }), [decision, kind]);
  return <GameContext.Provider value={context}><HoverProvider><DragProvider><CombatArrowProvider><TooltipProvider>
    <main className="audit-page setup-screen">
      <header><p className="audit-eyebrow">Ironsmith · component review</p><h1>Interface workshop</h1><p>Isolated fixtures use the production components and styles. Actions are recorded below; no real game is changed.</p></header>
      <section className="audit-section"><h2>Controls & states</h2>
        <div className="audit-controls">{["default", "secondary", "outline", "ghost", "destructive"].map(variant=><Button key={variant} variant={variant}>{variant}</Button>)}<Button disabled>Disabled</Button></div>
        <div className="audit-controls"><label>Card name<Input placeholder="Lightning Bolt"/></label><label className="audit-check"><Checkbox defaultChecked/>Selected</label><label className="audit-check"><Checkbox/>Unselected</label><Slider aria-label="Fidelity" defaultValue={[96]} className="w-40"/></div>
        <Tabs defaultValue="one"><TabsList><TabsTrigger value="one">Overview</TabsTrigger><TabsTrigger value="two">Details</TabsTrigger><TabsTrigger value="three" disabled>Unavailable</TabsTrigger></TabsList><TabsContent value="one">A clear selected tab and visible keyboard focus.</TabsContent><TabsContent value="two">Secondary content.</TabsContent></Tabs>
      </section>
      <section className="audit-section"><h2>Game decisions</h2>
        <div className="audit-controls"><label>Decision<select value={kind} onChange={event=>setKind(event.target.value)}>{Object.keys(fixtures).map(key=><option key={key}>{key}</option>)}</select></label><label>Presentation<select value={layout} onChange={event=>setLayout(event.target.value)}><option>panel</option><option>strip</option></select></label><label className="audit-check"><Checkbox checked={canAct} onCheckedChange={setCanAct}/>Can act</label></div>
        <div className={`audit-decision ${layout === "strip" ? "table-action-bar" : ""}`}><DecisionRouter key={`${kind}-${layout}`} decision={decision} canAct={canAct} layout={layout}/></div>
        <output aria-live="polite" className="audit-output">{lastAction}</output>
      </section>
      <section className="audit-section"><h2>Dialogs & long content</h2><Button onClick={()=>setDeckOpen(true)}>Open sample decklist</Button></section>
      <OpenDecklistModal decklist={deckOpen ? {playerName: "Alexandria, Keeper of the Very Long Player Name", deck: ["Island", "Island", "The Ultimate Nightmare of Wizards of the Coast® Customer Service", "Lightning Bolt"], sideboard: []} : null} onClose={()=>setDeckOpen(false)}/>
    </main>
  </TooltipProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider>;
}
createRoot(document.getElementById("root")).render(<I18nProvider><Audit/></I18nProvider>);
