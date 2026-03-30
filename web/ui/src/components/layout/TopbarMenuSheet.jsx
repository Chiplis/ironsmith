import { useEffect, useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import { copyTextToClipboard } from "@/lib/clipboard";
import { buildPuzzleUrlFromGameState } from "@/lib/puzzles";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { ExternalLink, Github, Menu, RefreshCw, Settings2 } from "lucide-react";
import AddCardSheet from "./AddCardSheet";
import CreateCardForgeSheet from "./CreateCardForgeSheet";

const inputClass =
  "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none disabled:cursor-not-allowed disabled:opacity-50";
const labelClass =
  "grid gap-1 text-[11px] uppercase tracking-[0.2em] text-muted-foreground";
const sectionClass =
  "fantasy-sheet-section grid gap-3 p-3";

function MenuSection({ eyebrow, title, description, children }) {
  return (
    <section className={sectionClass}>
      <div className="grid gap-1">
        <span className="text-[10px] uppercase tracking-[0.24em] text-[#c3a774]">
          {eyebrow}
        </span>
        <div className="text-[16px] font-bold uppercase tracking-[0.16em] text-foreground">
          {title}
        </div>
        {description ? (
          <p className="m-0 text-[13px] leading-5 text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {children}
    </section>
  );
}

export default function TopbarMenuSheet({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onChangePerspective,
  onRefresh,
  onToggleLog,
  onEnterDeckLoading,
  onOpenPuzzleSetup,
  onOpenLobby,
  deckLoadingMode,
  puzzleSetupMode = false,
  onAddCardNotice,
  triggerIcon = "settings",
  showQuickActions = false,
}) {
  const [open, setOpen] = useState(false);
  const { nonDesktopViewport } = useViewportLayout();
  const {
    state,
    wasmRegistryCount,
    wasmRegistryTotal,
    multiplayer,
    autoPassEnabled,
    setAutoPassEnabled,
    holdRule,
    setHoldRule,
    confirmEnabled,
    setConfirmEnabled,
    inspectorDebug,
    setInspectorDebug,
    setStatus,
  } = useGame();

  const players = state?.players || [];
  const perspective = state?.perspective;
  const me = players.find((player) => player.id === perspective) || players[0];
  const lobbyBusy = multiplayer.mode !== "idle";
  const matchLocked = multiplayer.matchStarted;
  const addLocked = multiplayer.mode !== "idle";
  const compiledLabel = useMemo(() => {
    if (!Number.isFinite(wasmRegistryCount) || wasmRegistryCount < 0) return "-";
    if (wasmRegistryTotal > 0) {
      return `${wasmRegistryCount.toLocaleString()}/${wasmRegistryTotal.toLocaleString()}`;
    }
    return wasmRegistryCount.toLocaleString();
  }, [wasmRegistryCount, wasmRegistryTotal]);
  const lobbyLabel = lobbyBusy
    ? `Lobby ${multiplayer.players.length}/${multiplayer.desiredPlayers || 0}`
    : "No lobby";

  const handleOpenLobby = () => {
    setOpen(false);
    onOpenLobby();
  };

  const handleToggleDeckLoading = () => {
    setOpen(false);
    onEnterDeckLoading();
  };
  const handleRefresh = () => {
    setOpen(false);
    onRefresh();
  };
  const handleOpenPuzzleSetup = () => {
    setOpen(false);
    onOpenPuzzleSetup();
  };
  const handleShareCurrentTable = async () => {
    setOpen(false);
    const shareUrl = buildPuzzleUrlFromGameState(state);
    if (!shareUrl) {
      setStatus("Could not build a puzzle link from the current table", true);
      return;
    }

    const copied = await copyTextToClipboard(shareUrl);
    setStatus(copied ? "Copied current table puzzle link" : "Could not copy puzzle link", !copied);
  };
  const handleToggleLog = () => {
    setOpen(false);
    onToggleLog();
  };
  const triggerGlyph = triggerIcon === "menu"
    ? <Menu className="size-3.5" />
    : <Settings2 className="size-3.5" />;
  const selectedPlayer = perspective ?? players[0]?.id ?? 0;
  const [forgePlayer, setForgePlayer] = useState(selectedPlayer);
  const [forgeZone, setForgeZone] = useState("battlefield");
  const [forgeSkipTriggers, setForgeSkipTriggers] = useState(false);

  useEffect(() => {
    setForgePlayer(selectedPlayer);
  }, [selectedPlayer]);

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button
          variant="secondary"
          size="icon-xs"
          className="stone-pill topbar-menu-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]"
          aria-label={triggerIcon === "menu" ? "Open navigation menu" : "Open game menu"}
        >
          {triggerGlyph}
        </Button>
      </SheetTrigger>
      <SheetContent
        side={nonDesktopViewport ? "center" : "right"}
        className="fantasy-sheet settings-sheet w-[min(94vw,420px)] overflow-y-auto p-0"
      >
        <SheetHeader className="fantasy-sheet-header pr-12">
          <div className="text-[11px] uppercase tracking-[0.24em] text-[#cdb27a]">Menu</div>
          <SheetTitle className="text-[22px] uppercase tracking-[0.18em] text-foreground">
            Table Settings
          </SheetTitle>
          <SheetDescription className="max-w-[32ch] text-[13px] leading-5">
            Match setup, diagnostics, and secondary info live here so the top bar can stay
            focused on gameplay.
          </SheetDescription>
        </SheetHeader>

        <div className="grid gap-4 p-4">
          {showQuickActions ? (
            <MenuSection
              eyebrow="Quick Actions"
              title="Shortcuts"
              description="Mobile keeps a single entry point, so the main desktop actions are gathered here."
            >
              <div className="grid gap-2 sm:grid-cols-2">
                <AddCardSheet
                  onAddCardNotice={onAddCardNotice}
                  trigger={(
                    <Button
                      variant="secondary"
                      size="sm"
                      className="stone-pill justify-start"
                      disabled={addLocked}
                    >
                      Add Card
                    </Button>
                  )}
                />
                <CreateCardForgeSheet
                  disabled={addLocked}
                  players={players}
                  selectedPlayer={forgePlayer}
                  onSelectPlayer={setForgePlayer}
                  zone={forgeZone}
                  onZoneChange={setForgeZone}
                  skipTriggers={forgeSkipTriggers}
                  onSkipTriggersChange={setForgeSkipTriggers}
                  trigger={(
                    <button
                      type="button"
                      className="stone-pill inline-flex items-center justify-start rounded-none px-2.5 py-2 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45"
                      disabled={addLocked}
                    >
                      Create Card
                    </button>
                  )}
                />
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  disabled={lobbyBusy}
                  onClick={handleToggleDeckLoading}
                >
                  {deckLoadingMode ? "Cancel Deck Load" : "Load Decks"}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  disabled={lobbyBusy}
                  onClick={handleOpenPuzzleSetup}
                >
                  {puzzleSetupMode ? "Close Puzzle" : "Puzzle Setup"}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  onClick={handleShareCurrentTable}
                >
                  Share Table
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  onClick={handleOpenLobby}
                >
                  {lobbyBusy ? "Open Lobby" : "Create Lobby"}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  onClick={handleToggleLog}
                >
                  Open Log
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  onClick={handleRefresh}
                >
                  <RefreshCw className="size-3.5" />
                  Refresh View
                </Button>
              </div>
              <Button variant="secondary" size="sm" className="stone-pill justify-start" asChild>
                <a
                  href="https://github.com/Chiplis/ironsmith"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <Github className="size-3.5" />
                  Repository
                  <ExternalLink className="size-3" />
                </a>
              </Button>
            </MenuSection>
          ) : null}
          <MenuSection
            eyebrow="Match"
            title="Setup"
            description="Pre-game configuration and table management. These controls stay out of the way once the match is underway."
          >
            <div className="grid gap-3 sm:grid-cols-2">
              <label className={labelClass}>
                Player Names
                <input
                  className={inputClass}
                  value={playerNames}
                  disabled={lobbyBusy}
                  onChange={(event) => setPlayerNames(event.target.value)}
                />
              </label>
              <label className={labelClass}>
                Starting Life
                <input
                  className={inputClass}
                  type="number"
                  min={1}
                  value={startingLife}
                  disabled={lobbyBusy}
                  onChange={(event) => setStartingLife(Number(event.target.value) || 20)}
                />
              </label>
            </div>
            <div className="grid gap-2 sm:grid-cols-2">
              <Button
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={lobbyBusy}
                onClick={onReset}
              >
                Reset Match
              </Button>
              <Button
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={lobbyBusy}
                onClick={handleToggleDeckLoading}
              >
                {deckLoadingMode ? "Cancel Deck Load" : "Load Decks"}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={lobbyBusy}
                onClick={handleOpenPuzzleSetup}
              >
                {puzzleSetupMode ? "Close Puzzle" : "Puzzle Setup"}
              </Button>
              <Button variant="secondary" size="sm" className="stone-pill" onClick={handleShareCurrentTable}>
                Share Table
              </Button>
              <Button variant="secondary" size="sm" className="stone-pill" onClick={handleOpenLobby}>
                {lobbyBusy ? "Open Lobby" : "Create Lobby"}
              </Button>
              <Button variant="secondary" size="sm" className="stone-pill" onClick={handleRefresh}>
                <RefreshCw className="size-3.5" />
                Refresh View
              </Button>
            </div>
          </MenuSection>

          <MenuSection
            eyebrow="View"
            title="Perspective"
            description="UI-side controls that affect what you are inspecting, not the game rules themselves."
          >
            <label className={labelClass}>
              Playing As
              <select
                className={inputClass}
                value={perspective ?? ""}
                disabled={matchLocked}
                onChange={(event) => onChangePerspective(Number(event.target.value))}
              >
                {players.map((player) => (
                  <option key={player.id} value={player.id}>
                    {player.name}
                  </option>
                ))}
              </select>
            </label>
          </MenuSection>

          <MenuSection
            eyebrow="Info"
            title="Session"
            description="Reference info that does not need to sit in the gameplay lane."
          >
            <div className="grid gap-2 text-[13px] text-foreground">
              <div className="fantasy-sheet-stat flex items-center justify-between gap-3 px-3 py-2">
                <span className="uppercase tracking-[0.16em] text-muted-foreground">View</span>
                <Badge variant="secondary" className="fantasy-sheet-badge text-[12px] uppercase">
                  {me?.name || "-"}
                </Badge>
              </div>
              <div className="fantasy-sheet-stat flex items-center justify-between gap-3 px-3 py-2">
                <span className="uppercase tracking-[0.16em] text-muted-foreground">
                  Cards Compiled
                </span>
                <Badge variant="secondary" className="fantasy-sheet-badge text-[12px] uppercase">
                  {compiledLabel}
                </Badge>
              </div>
              <div className="fantasy-sheet-stat flex items-center justify-between gap-3 px-3 py-2">
                <span className="uppercase tracking-[0.16em] text-muted-foreground">Lobby</span>
                <Badge variant="secondary" className="fantasy-sheet-badge text-[12px] uppercase">
                  {lobbyLabel}
                </Badge>
              </div>
            </div>
            <Button variant="secondary" size="sm" className="stone-pill" asChild>
              <a
                href="https://github.com/Chiplis/ironsmith"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Github className="size-3.5" />
                Repository
                <ExternalLink className="size-3" />
              </a>
            </Button>
          </MenuSection>

          <MenuSection
            eyebrow="Live"
            title="Gameplay Controls"
            description="Controls moved out of the landscape gameplay lane so the table can stay compact on mobile."
          >
            <div className="grid gap-3 sm:grid-cols-2">
              <label className={labelClass}>
                Auto-pass Hold
                <select
                  className={inputClass}
                  value={holdRule}
                  disabled={matchLocked}
                  onChange={(event) => setHoldRule(event.target.value)}
                >
                  <option value="never">Never</option>
                  <option value="if_actions">If actions</option>
                  <option value="stack">Stack</option>
                  <option value="main">Main</option>
                  <option value="combat">Combat</option>
                  <option value="ending">Ending</option>
                  <option value="always">Always</option>
                </select>
              </label>
              <div className="grid gap-2">
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={autoPassEnabled}
                    disabled={matchLocked}
                    onCheckedChange={(value) => setAutoPassEnabled(Boolean(value))}
                  />
                  Auto-pass
                </label>
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={confirmEnabled}
                    onCheckedChange={(value) => setConfirmEnabled(Boolean(value))}
                  />
                  Confirm Stops
                </label>
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={inspectorDebug}
                    onCheckedChange={(value) => setInspectorDebug(Boolean(value))}
                  />
                  Debug
                </label>
              </div>
            </div>
            <Button variant="secondary" size="sm" className="stone-pill" onClick={handleToggleLog}>
              Open Log
            </Button>
          </MenuSection>
        </div>
      </SheetContent>
    </Sheet>
  );
}
