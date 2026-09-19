import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { GameContext } from "../src/context/GameContext.shared";
import { I18nProvider } from "../src/i18n/I18nContext";
import OpenDecklistModal from "../src/components/board/OpenDecklistModal";
import LogDrawer from "../src/components/overlays/LogDrawer";
import RandomGameSheet from "../src/components/layout/RandomGameSheet";
import AddCardSheet from "../src/components/layout/AddCardSheet";
import DiagnosticsSheet from "../src/components/layout/DiagnosticsSheet";
import VerifyMatchSheet from "../src/components/layout/VerifyMatchSheet";
import TopbarMenuSheet from "../src/components/layout/TopbarMenuSheet";
import LobbyOverlay from "../src/components/layout/LobbyOverlay";
import CreateCardForgeSheet from "../src/components/layout/CreateCardForgeSheet";
import "../src/index.css";

const players = [
  { id: 0, name: "Alice", life: 20, hand_size: 7, library_size: 53, battlefield: [], graveyard_cards: [], exile_cards: [] },
  { id: 1, name: "Bob", life: 20, hand_size: 7, library_size: 53, battlefield: [], graveyard_cards: [], exile_cards: [] },
];

const CLOCK = Object.freeze({ elapsedMs: 0, running: false, startedAt: 0 });
const clockStore = { subscribe: () => () => {}, getSnapshot: () => CLOCK };

const ctx = {
  state: { players, perspective: 0, turn: 3, phase: "Main", decision: { kind: "priority", player: 0 } },
  game: { ziffleVerifyShuffle: () => ({}) },
  multiplayer: {
    mode: "idle", role: null, matchStarted: false, peers: [], players: [],
    connected: true, lastAppliedSequence: 0, signalingServer: "0.peerjs.com:443",
  },
  matchClockStore: clockStore,
  logEntries: [
    { time: "12:00:01", message: "Alice plays Island." },
    { time: "12:00:04", message: "Bob casts Counterspell." },
    { time: "12:00:09", message: "Illegal action rejected.", isError: true },
  ],
  setStatus: () => {},
  addCard: () => {},
  settings: {},
  perspectivePlayerIndex: 0,
  status: "",
  auditTranscript: null,
  canStartHostedMatch: false,
  createLobby: () => {}, joinLobby: () => {}, leaveLobby: () => {},
  startHostedMatch: () => {}, updateLobbyDeck: () => {},
  exportAuditTranscript: () => "", replayAuditTranscript: () => ({}),
  auditReplay: null,
  prepareAuditReplaySession: () => {}, beginAuditReplaySession: () => {},
  setAuditReplayPosition: () => {}, exitAuditReplaySession: () => {},
  runWasmInteraction: (fn) => fn?.(),
  updateSettings: () => {},
};

const DECKLIST = {
  playerName: "Alice",
  deck: ["Lightning Bolt", "Lightning Bolt", "Island", "Island", "Counterspell"],
  sideboard: ["Pyroblast", "Tormod's Crypt"],
  commanders: [],
};

function Fixture() {
  const [which, setWhich] = useState("");
  const close = () => setWhich("");
  const trigger = (id, label) => (
    <button type="button" data-open={id} onClick={() => setWhich(id)}>{label}</button>
  );
  return (
    <I18nProvider>
      <GameContext.Provider value={ctx}>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 12 }}>
          {trigger("decklist", "Decklist")}
          {trigger("log", "Log")}
          <RandomGameSheet trigger={<button type="button" data-open="random">Random</button>} onGenerate={() => {}} />
          <AddCardSheet trigger={<button type="button" data-open="addcard">Add Card</button>} />
          <DiagnosticsSheet trigger={<button type="button" data-open="diagnostics">Diagnostics</button>} />
          <VerifyMatchSheet trigger={<button type="button" data-open="verify" className="verify-trigger">Verify</button>} />
          <CreateCardForgeSheet players={players} trigger={<button type="button" data-open="forge">Forge</button>} />
          {trigger("lobby", "Lobby")}
          <TopbarMenuSheet playerNames={["Alice", "Bob"]} setPlayerNames={() => {}} startingLife={20} setStartingLife={() => {}} />
        </div>
        {which === "decklist" ? <OpenDecklistModal decklist={DECKLIST} onClose={close} /> : null}
        <LogDrawer open={which === "log"} onOpenChange={(open) => setWhich(open ? "log" : "")} />
        {which === "lobby" ? <LobbyOverlay onClose={close} /> : null}
      </GameContext.Provider>
    </I18nProvider>
  );
}

createRoot(document.getElementById("root")).render(<Fixture />);
