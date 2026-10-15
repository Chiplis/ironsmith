import { Github } from "lucide-react";
import useUiText from "@/i18n/useUiText";
import { Button } from "@/components/ui/button";
import TopbarMenuSheet from "./TopbarMenuSheet";

export default function TopbarUtilityControls({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onRefresh,
  onToggleLog,
  onOpenPuzzleSetup,
  onOpenLobby,
  onGenerateRandomGame,
  puzzleSetupMode = false,
  onAddCardNotice,
  showBrand = false,
}) {
  const ui = useUiText();
  return (
    <div className="topbar-minor-controls topbar-minor-controls--utility">
      {showBrand ? (
        <h1 className="toolbar-brand topbar-brand m-0 whitespace-nowrap font-bold">Ironsmith</h1>
      ) : null}
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
          aria-label={ui("Ironsmith on GitHub")}
          title="GitHub"
        >
          <Github className="size-3.5" aria-hidden="true" />
        </a>
      </Button>
      <Button
        variant="secondary"
        size="icon-xs"
        className="stone-pill topbar-discord-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]"
        asChild
      >
        <a
          href="https://discord.gg/XDa4npdpVE"
          target="_blank"
          rel="noopener noreferrer"
          aria-label={ui("Ironsmith on Discord")}
          title="Discord"
        >
          <svg className="size-3.5" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.211.375-.445.864-.608 1.249a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.249.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128c.126-.094.252-.192.372-.291a.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.099.246.197.372.291a.077.077 0 0 1-.006.128c-.597.35-1.22.648-1.873.892a.076.076 0 0 0-.04.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.548-13.66a.061.061 0 0 0-.031-.03ZM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.211 0 2.176 1.096 2.157 2.419 0 1.334-.955 2.419-2.157 2.419Zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.211 0 2.176 1.096 2.157 2.419 0 1.334-.946 2.419-2.157 2.419Z" />
          </svg>
        </a>
      </Button>
      <TopbarMenuSheet
        playerNames={playerNames}
        setPlayerNames={setPlayerNames}
        startingLife={startingLife}
        setStartingLife={setStartingLife}
        onReset={onReset}
        onRefresh={onRefresh}
        onToggleLog={onToggleLog}
        onOpenPuzzleSetup={onOpenPuzzleSetup}
        onOpenLobby={onOpenLobby}
        onGenerateRandomGame={onGenerateRandomGame}
        puzzleSetupMode={puzzleSetupMode}
        onAddCardNotice={onAddCardNotice}
        triggerIcon="menu"
      />
    </div>
  );
}
