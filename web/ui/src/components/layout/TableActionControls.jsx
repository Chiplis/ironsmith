import { useState } from "react";
import { Download } from "lucide-react";
import { useGame } from "@/context/GameContext";
import { copyTextToClipboard } from "@/lib/clipboard";
import { buildPuzzleUrlFromGameState } from "@/lib/puzzles";
import CreateCardForgeSheet from "./CreateCardForgeSheet";
import AddCardSheet from "./AddCardSheet";
import AuditReplayControls from "./AuditReplayControls";
import VerifyMatchSheet from "./VerifyMatchSheet";
import { useI18n } from "@/i18n/I18nContext";

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
    exportAuditTranscript,
  } = useGame();
  const { t } = useI18n();
  const [zone, setZone] = useState("hand");
  const [playerIndex, setPlayerIndex] = useState(null);
  const [skipTriggers, setSkipTriggers] = useState(false);

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;
  const selectedPlayer = playerIndex ?? perspective;
  const addLocked = multiplayer.mode !== "idle" && !multiplayer.matchStarted;
  const lobbyBusy = multiplayer.mode !== "idle";
  const canExportMatch = Boolean(
    typeof exportAuditTranscript === "function"
    && (state?.game_over || multiplayer?.matchDisputed || multiplayer?.mode === "disputed")
  );

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

  return (
    <div className="table-zone-action-controls" aria-label={t("settings.quick.eyebrow")}>
      <VerifyMatchSheet />
      <AuditReplayControls />
      {canExportMatch ? (
        <button
          type="button"
          className={triggerPill}
          onClick={handleExportMatch}
        >
          <Download className="size-3.5" aria-hidden="true" />
          {t("action.exportMatch")}
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
            {t("action.addCard")}
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
            {t("action.compileCard")}
          </button>
        )}
      />
      <button
        type="button"
        className={triggerPill}
        disabled={lobbyBusy}
        onClick={onEnterDeckLoading}
      >
        {deckLoadingMode ? t("action.cancelDeckLoad") : t("action.loadDecks")}
      </button>
      {!compact ? (
        <button
          type="button"
          className={triggerPill}
          disabled={lobbyBusy}
          onClick={onOpenPuzzleSetup}
        >
          {puzzleSetupMode ? t("action.closePuzzle") : t("action.puzzleSetup")}
        </button>
      ) : null}
      {!compact ? (
        <button
          type="button"
          className={triggerPill}
          onClick={handleShareCurrentTable}
        >
          {t("action.shareTable")}
        </button>
      ) : null}
      <button
        type="button"
        className={triggerPill}
        onClick={onOpenLobby}
      >
        {lobbyBusy ? t("action.openLobby") : t("action.createLobby")}
      </button>
    </div>
  );
}
