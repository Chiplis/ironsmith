import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { copyTextToClipboard } from "@/lib/clipboard";
import { buildPuzzleUrlFromGameState } from "@/lib/puzzles";
import CreateCardForgeSheet from "./CreateCardForgeSheet";
import AddCardSheet from "./AddCardSheet";

const triggerPill = "stone-pill table-zone-action-button inline-flex items-center justify-center rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45";

export default function TableActionControls({
  compact = false,
  onAddCardNotice,
  onEnterDeckLoading,
  onOpenPuzzleSetup,
  onOpenLobby,
  deckLoadingMode = false,
  puzzleSetupMode = false,
}) {
  const {
    state,
    multiplayer,
    setStatus,
  } = useGame();
  const [zone, setZone] = useState("hand");
  const [playerIndex, setPlayerIndex] = useState(null);
  const [skipTriggers, setSkipTriggers] = useState(false);

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;
  const selectedPlayer = playerIndex ?? perspective;
  const addLocked = multiplayer.mode !== "idle";
  const lobbyBusy = multiplayer.mode !== "idle";

  const handleShareCurrentTable = async () => {
    const shareUrl = buildPuzzleUrlFromGameState(state);
    if (!shareUrl) {
      setStatus("Could not build a puzzle link from the current table", true);
      return;
    }

    const copied = await copyTextToClipboard(shareUrl);
    setStatus(copied ? "Copied current table puzzle link" : "Could not copy puzzle link", !copied);
  };

  return (
    <div className="table-zone-action-controls" aria-label="Table actions">
      <AddCardSheet
        onAddCardNotice={onAddCardNotice}
        trigger={(
          <button
            type="button"
            className={triggerPill}
            disabled={addLocked}
          >
            Add Card
          </button>
        )}
      />
      <CreateCardForgeSheet
        disabled={addLocked}
        players={players}
        selectedPlayer={selectedPlayer}
        onSelectPlayer={setPlayerIndex}
        zone={zone}
        onZoneChange={setZone}
        skipTriggers={skipTriggers}
        onSkipTriggersChange={(checked) => setSkipTriggers(checked === true)}
        trigger={(
          <button
            type="button"
            className={triggerPill}
            disabled={addLocked}
          >
            Create Card
          </button>
        )}
      />
      <button
        type="button"
        className={triggerPill}
        disabled={lobbyBusy}
        onClick={onEnterDeckLoading}
      >
        {deckLoadingMode ? "Cancel Deck Load" : "Load Decks"}
      </button>
      {!compact ? (
        <button
          type="button"
          className={triggerPill}
          disabled={lobbyBusy}
          onClick={onOpenPuzzleSetup}
        >
          {puzzleSetupMode ? "Close Puzzle" : "Puzzle Setup"}
        </button>
      ) : null}
      {!compact ? (
        <button
          type="button"
          className={triggerPill}
          onClick={handleShareCurrentTable}
        >
          Share Table
        </button>
      ) : null}
      <button
        type="button"
        className={triggerPill}
        onClick={onOpenLobby}
      >
        {lobbyBusy ? "Open Lobby" : "Create Lobby"}
      </button>
    </div>
  );
}
