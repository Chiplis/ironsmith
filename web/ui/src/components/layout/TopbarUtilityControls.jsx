import useUiText from "@/i18n/useUiText";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { Button } from "@/components/ui/button";
import { Bug, Github, ScrollText } from "lucide-react";
import TopbarMenuSheet from "./TopbarMenuSheet";

export default function TopbarUtilityControls({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onRefresh,
  onToggleLog,
  onEnterDeckLoading,
  onOpenPuzzleSetup,
  onOpenLobby,
  deckLoadingMode,
  puzzleSetupMode = false,
  onAddCardNotice,
  showInlineControls = true,
  children,
}) {
  const ui = useUiText();
  const { inspectorDebug, setInspectorDebug } = useGame();
  const { t } = useI18n();
  return (
    <div className="topbar-minor-controls topbar-minor-controls--utility">
      {showInlineControls ? (
        <Button
          variant="secondary"
          size="icon-xs"
          className="stone-pill topbar-github-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]"
          asChild
        >
          <a
            href="https://github.com/Chiplis/ironsmith"
            target="_blank"
            rel="noopener noreferrer"
            aria-label={t("settings.repository")}
            title="GitHub"
          >
            <Github className="size-3.5" />
          </a>
        </Button>
      ) : null}
      <TopbarMenuSheet
        playerNames={playerNames}
        setPlayerNames={setPlayerNames}
        startingLife={startingLife}
        setStartingLife={setStartingLife}
        onReset={onReset}
        onRefresh={onRefresh}
        onToggleLog={onToggleLog}
        onEnterDeckLoading={onEnterDeckLoading}
        onOpenPuzzleSetup={onOpenPuzzleSetup}
        onOpenLobby={onOpenLobby}
        deckLoadingMode={deckLoadingMode}
        puzzleSetupMode={puzzleSetupMode}
        onAddCardNotice={onAddCardNotice}
        triggerIcon={showInlineControls ? "settings" : "menu"}
        showQuickActions={!showInlineControls}
      />
      {showInlineControls ? (
        <>
          <Button
            variant="secondary"
            size="icon-xs"
            className="stone-pill topbar-log-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]"
            onClick={onToggleLog}
            aria-label={t("settings.openLog")}
            title={t("settings.openLog")}
          >
            <ScrollText className="size-3.5" />
          </Button>
          <Button
            variant="secondary"
            size="icon-xs"
            className={`stone-pill topbar-debug-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]${inspectorDebug ? " is-active" : ""}`}
            onClick={() => setInspectorDebug(!inspectorDebug)}
            aria-label={t("settings.debug")}
            aria-pressed={inspectorDebug}
            title={ui(inspectorDebug ? t("settings.debugEnabled") : t("settings.debug"))}
          >
            <Bug className="size-3.5" />
          </Button>
        </>
      ) : null}
      {children}
    </div>
  );
}
