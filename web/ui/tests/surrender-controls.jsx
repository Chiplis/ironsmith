import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { HoverProvider } from "../src/context/HoverContext";
import { I18nProvider } from "../src/i18n/I18nContext";
import { TooltipProvider } from "../src/components/ui/tooltip";
import DecisionPanel from "../src/components/left-rail/DecisionPanel";
import CardCreationControls from "../src/components/layout/CardCreationControls";
import "../src/index.css";

function Fixture() {
  const [surrenderRequested, setSurrenderRequested] = useState(false);
  const [commands, setCommands] = useState([]);
  const [holdRule, setHoldRule] = useState("never");
  const query = new URLSearchParams(location.search);
  const mode = query.get("mode") || "trusted";
  const state = {
    perspective: 1, active_player: 0, phase: "Combat", stack_size: 2,
    players: [{ id: 0, name: "Host" }, { id: 1, name: "Guest" }],
    decision: query.has("waiting") ? null : {
      kind: "priority", player: 0,
      actions: [{ index: 0, kind: "pass_priority", label: "Pass priority" }],
    },
  };
  return <I18nProvider><GameContext.Provider value={{
    state, multiplayer: { matchStarted: true, mode: "in_match", localPlayerIndex: 1, securityMode: mode },
    surrenderRequested, setSurrenderRequested, holdRule, setHoldRule,
    dispatch: () => { throw new Error("Confirmation must not dispatch the pending decision"); },
    submitMultiplayerCommand: async command => setCommands(prev => [...prev, command]),
    setStatus: console.error,
  }}><TooltipProvider><HoverProvider>
    <main style={{ width: 320, height: 300 }}>
      <CardCreationControls />
      <DecisionPanel />
    </main>
    <output>{JSON.stringify(commands)}</output>
  </HoverProvider></TooltipProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById("root")).render(<Fixture />);
