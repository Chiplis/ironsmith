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
  return (
    <div className="topbar-minor-controls topbar-minor-controls--utility">
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
        triggerIcon="menu"
        showQuickActions={!showInlineControls && !children}
        tableTools={children}
      />
    </div>
  );
}
