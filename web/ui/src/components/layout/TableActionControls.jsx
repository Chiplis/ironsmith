import { useRef, useState } from "react";
import { Download, ShieldCheck } from "lucide-react";
import { useGame } from "@/context/GameContext";
import { copyTextToClipboard } from "@/lib/clipboard";
import { buildPuzzleUrlFromGameState } from "@/lib/puzzles";
import { verifyLiveAuditTranscript } from "@/lib/multiplayer-audit";
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
    game,
    state,
    multiplayer,
    setStatus,
    exportAuditTranscript,
  } = useGame();
  const [zone, setZone] = useState("hand");
  const [playerIndex, setPlayerIndex] = useState(null);
  const [skipTriggers, setSkipTriggers] = useState(false);
  const verifyInputRef = useRef(null);

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;
  const selectedPlayer = playerIndex ?? perspective;
  const addLocked = multiplayer.mode !== "idle" && !multiplayer.matchStarted;
  const lobbyBusy = multiplayer.mode !== "idle";
  const canExportMatch = Boolean(
    typeof exportAuditTranscript === "function"
    && (state?.game_over || multiplayer?.matchDisputed || multiplayer?.mode === "disputed")
  );
  const canVerifyMatch = Boolean(game);

  const verifyExportedShuffleProof = async (proof) => {
    if (!game || typeof game.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    const verified = await game.ziffleVerifyShuffle({
      deckCount: Number(proof.deckCount),
      context: String(proof.context || ""),
      keyContext: String(proof.keyContext || proof.context || ""),
      keys: proof.keys || [],
      steps: proof.steps || [],
    });
    if (String(verified.deckHash || "") !== String(proof.deckHash || "")) {
      throw new Error(`Ziffle shuffle proof mismatch for player ${Number(proof.owner) + 1}`);
    }
  };

  const describeVerificationReport = (report) => {
    const actions = Number(report?.verifiedActions || 0);
    const suffix = `${actions} action${actions === 1 ? "" : "s"}`;
    const outcome = report?.outcome || {};
    if (outcome.status === "winner") {
      const winner = outcome.winnerName || `Player ${Number(outcome.winner) + 1}`;
      return `Match verified: ${winner} wins (${suffix})`;
    }
    if (outcome.status === "draw") {
      return `Match verified: draw (${suffix})`;
    }
    if (outcome.status === "disputed") {
      const accused = (outcome.accusedPlayers || [])
        .map((player) => `Player ${Number(player) + 1}`)
        .join(", ");
      return `Match verified: disputed${accused ? `; evidence implicates ${accused}` : ""} (${suffix})`;
    }
    return `Match verified: stalled or incomplete (${suffix})`;
  };

  const handleShareCurrentTable = async () => {
    const shareUrl = buildPuzzleUrlFromGameState(state);
    if (!shareUrl) {
      setStatus("Could not build a puzzle link from the current table", true);
      return;
    }

    const copied = await copyTextToClipboard(shareUrl);
    setStatus(copied ? "Copied current table puzzle link" : "Could not copy puzzle link", !copied);
  };

  const handleExportMatch = async () => {
    if (typeof exportAuditTranscript !== "function") {
      setStatus("No match transcript is available to export", true);
      return;
    }
    try {
      const transcript = await exportAuditTranscript();
      if (!transcript) {
        setStatus("No match transcript is available to export", true);
        return;
      }
      const matchId = String(transcript.matchId || transcript.match?.auditMatchId || "match")
        .replace(/[^a-z0-9._-]+/gi, "-")
        .replace(/^-+|-+$/g, "")
        || "match";
      const blob = new Blob([`${JSON.stringify(transcript, null, 2)}\n`], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `ironsmith-${matchId}-audit.json`;
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
      setStatus("Exported match audit transcript");
    } catch (err) {
      setStatus(`Export match failed: ${err?.message || err}`, true);
    }
  };

  const handleVerifyMatchClick = () => {
    verifyInputRef.current?.click();
  };

  const handleVerifyMatchFile = async (event) => {
    const file = event.target.files?.[0] || null;
    event.target.value = "";
    if (!file) return;
    try {
      const transcript = JSON.parse(await file.text());
      const report = await verifyLiveAuditTranscript(
        transcript,
        globalThis.crypto,
        { verifyShuffleProof: verifyExportedShuffleProof }
      );
      setStatus(describeVerificationReport(report));
    } catch (err) {
      setStatus(`Verify match failed: ${err?.message || err}`, true);
    }
  };

  return (
    <div className="table-zone-action-controls" aria-label="Table actions">
      <input
        ref={verifyInputRef}
        type="file"
        accept="application/json,.json"
        className="hidden"
        onChange={handleVerifyMatchFile}
      />
      <button
        type="button"
        className={triggerPill}
        disabled={!canVerifyMatch}
        onClick={handleVerifyMatchClick}
      >
        <ShieldCheck className="size-3.5" aria-hidden="true" />
        Verify match
      </button>
      {canExportMatch ? (
        <button
          type="button"
          className={triggerPill}
          onClick={handleExportMatch}
        >
          <Download className="size-3.5" aria-hidden="true" />
          Export match
        </button>
      ) : null}
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
            Compile Card
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
