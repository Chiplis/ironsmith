import { useGame } from "@/context/GameContext";
import { copyTextToClipboard } from "@/lib/clipboard";
import { buildPuzzleUrlFromGameState } from "@/lib/puzzles";
import CardCreationControls from "./CardCreationControls";
import RandomGameSheet from "./RandomGameSheet";
import AuditReplayControls from "./AuditReplayControls";
import VerifyMatchSheet from "./VerifyMatchSheet";
import ExportMatchButton from "./ExportMatchButton";
import { useI18n } from "@/i18n/I18nContext";

const triggerPill = "stone-pill table-zone-action-button inline-flex items-center justify-center rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45";

export default function TableActionControls({
  compact = false,
  onAddCardNotice,
  onOpenPuzzleSetup,
  onGenerateRandomGame,
  onOpenLobby,
  puzzleSetupMode = false,
}) {
  const {
    state,
    multiplayer,
    setStatus,
  } = useGame();
  const { t } = useI18n();
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
    <div className="table-zone-action-controls" aria-label={t("settings.tableTools.eyebrow")}>
      <VerifyMatchSheet />
      <AuditReplayControls />
      <ExportMatchButton className={triggerPill} />
      <CardCreationControls onAddCardNotice={onAddCardNotice} />
      <RandomGameSheet
        onGenerate={onGenerateRandomGame}
        disabled={lobbyBusy}
        trigger={(
          <button
            type="button"
            className={triggerPill}
            disabled={lobbyBusy}
          >
            {t("action.randomGame")}
          </button>
        )}
      />
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
