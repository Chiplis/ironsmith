import usePrioritySettings from "@/hooks/usePrioritySettings";
import useGraphicsQuality from '@/hooks/useGraphicsQuality';
import useBattlefieldAppearance from '@/hooks/useBattlefieldAppearance';
import useUiText from "@/i18n/useUiText";
import { useEffect, useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useSuppressCardPreview } from "@/context/HoverContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import { copyTextToClipboard } from "@/lib/clipboard";
import { buildPuzzleUrlFromGameState } from "@/lib/puzzles";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Slider } from "@/components/ui/slider";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Activity, ExternalLink, UserPlus, X, Gamepad2, Github, Keyboard, LayoutGrid, Menu, Puzzle, RefreshCw, ScrollText, Settings2, Share2, ShieldCheck, Shuffle, Users, Wrench } from "lucide-react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import AddCardSheet from "./AddCardSheet";
import CustomCardsButton from "./CustomCardsButton";
import SurrenderButton from "./SurrenderButton";
import CreateCardForgeSheet from "./CreateCardForgeSheet";
import VerifyMatchSheet from "./VerifyMatchSheet";
import DiagnosticsSheet from "./DiagnosticsSheet";
import RandomGameSheet from "./RandomGameSheet";
import AuditReplayControls from "./AuditReplayControls";
import ExportMatchButton from "./ExportMatchButton";
import "./topbar-menu-tabs.css";
import { playerDisplayName } from "@/lib/player-display";
import { getPlayerAccent } from "@/lib/player-colors";
import { UI_FONT_OPTIONS } from "@/lib/ui-fonts";
import { useI18n } from "@/i18n/I18nContext";
import { openKeyboardShortcuts, useShowShortcutsOnStart } from "@/lib/shortcuts-overlay";
import { MAX_TABLE_SEATS, MIN_TABLE_SEATS, defaultSeatName } from "@/lib/table-setup";

const inputClass =
  "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none disabled:cursor-not-allowed disabled:opacity-50";
const labelClass =
  "grid gap-1 text-[11px] uppercase tracking-[0.2em] text-muted-foreground";
const sectionClass =
  "fantasy-sheet-section settings-sheet-section grid gap-3 py-5";

const MENU_TABS = [
  { value: "play", label: "Play", Icon: Gamepad2 },
  { value: "table", label: "Table", Icon: LayoutGrid },
  { value: "settings", label: "Settings", Icon: Settings2 },
  { value: "developer", label: "Developer", Icon: Wrench },
];
const MENU_TAB_STORAGE_KEY = "ironsmith.menuTab";

function readMenuTab() {
  try {
    const saved = localStorage.getItem(MENU_TAB_STORAGE_KEY);
    return MENU_TABS.some(entry => entry.value === saved) ? saved : "play";
  } catch {
    return "play";
  }
}

function MenuSection({ eyebrow, title, description, children, className = "" }) {
  const ui = useUiText();
  return (
    <section className={`${sectionClass} ${className}`}>
      <div className="grid gap-1">
        <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">
          {ui(eyebrow)}
        </span>
        <div className="text-[13px] font-bold uppercase tracking-[0.16em] text-[#e7d9bc]">
          {ui(title)}
        </div>
        {description ? (
          <p className="m-0 text-[13px] leading-5 text-muted-foreground">{ui(description)}</p>
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
  onRefresh,
  onToggleLog,
  onOpenPuzzleSetup,
  onOpenLobby,
  onGenerateRandomGame = null,
  puzzleSetupMode = false,
  onAddCardNotice,
  triggerIcon = "settings",
}) {
  const ui = useUiText();
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState(readMenuTab);
  const selectTab = (value) => {
    setTab(value);
    try { localStorage.setItem(MENU_TAB_STORAGE_KEY, value); } catch { /* Tab memory is a convenience. */ }
  };
  useSuppressCardPreview(open);
  const { nonDesktopViewport } = useViewportLayout();
  const {
    state,
    wasmRegistryCount,
    wasmRegistryTotal,
    multiplayer,
    priorityPolicy,
    fixedStartingBoard,
    setFixedStartingBoard,
    inspectorDebug,
    setInspectorDebug,
    setStatus,
    semanticThreshold,
    setSemanticThreshold,
    cardsMeetingThreshold,
    uiFont,
    setUiFont,
    playerAccentOverrides,
    setPlayerAccentOverride,
    followActivePlayer,
    setFollowActivePlayer,
  } = useGame();
  const priorityStatus = usePrioritySettings();
  const [showShortcutsOnStart, setShowShortcutsOnStart] = useShowShortcutsOnStart();
  const { locale, locales, setLocale, t } = useI18n();
  const [graphicsQuality, setGraphicsQuality] = useGraphicsQuality();
  const [battlefieldAppearance, setBattlefieldAppearance] = useBattlefieldAppearance();

  const players = useMemo(() => state?.players || [], [state?.players]);
  const perspective = state?.perspective;
  const me = players.find((player) => player.id === perspective) || players[0];
  const perspectiveAccent = getPlayerAccent(
    players,
    perspective,
    perspective ?? 0,
    playerAccentOverrides
  );
  // Raw slots keep a blank name editable; Reset gives it a default name.
  const playerNameSlots = useMemo(() => {
    const slots = String(playerNames ?? "").split(",").map((name) => name.trim()).slice(0, MAX_TABLE_SEATS);
    while (slots.length < MIN_TABLE_SEATS) slots.push("");
    return slots;
  }, [playerNames]);
  const seatCount = playerNameSlots.length;
  const commanderTable = seatCount >= 3;
  const lobbyBusy = multiplayer.mode !== "idle";
  const inMatch = Boolean(multiplayer.matchStarted || multiplayer.mode === "in_match");
  const addLocked = multiplayer.mode !== "idle" && !multiplayer.matchStarted;
  const compiledLabel = useMemo(() => {
    if (!Number.isFinite(wasmRegistryCount) || wasmRegistryCount < 0) return "-";
    if (wasmRegistryTotal > 0) {
      return `${wasmRegistryCount.toLocaleString()}/${wasmRegistryTotal.toLocaleString()}`;
    }
    return wasmRegistryCount.toLocaleString();
  }, [wasmRegistryCount, wasmRegistryTotal]);
  const lobbyLabel = lobbyBusy
    ? `Lobby ${multiplayer.players.length}/${multiplayer.desiredPlayers || 0}`
    : t("settings.noLobby");
  const connectionWarnings = multiplayer.connectionWarnings || [];
  const offlinePlayers = connectionWarnings.filter((warning) => !warning.local);
  const updatePlayerName = (index, value) => {
    const nextNames = [...playerNameSlots];
    // Commas separate seats in the saved setup, so they cannot be part of a name.
    nextNames[index] = value.replace(/,/g, "");
    setPlayerNames(nextNames.join(","));
  };
  const addPlayer = () => {
    if (seatCount >= MAX_TABLE_SEATS) return;
    setPlayerNames([...playerNameSlots, defaultSeatName(seatCount)].join(","));
  };
  const removePlayer = (index) => {
    if (seatCount <= MIN_TABLE_SEATS) return;
    setPlayerNames(playerNameSlots.filter((_, slot) => slot !== index).join(","));
  };

  const handleOpenLobby = () => {
    setOpen(false);
    onOpenLobby();
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
        {triggerIcon === "brand" ? (
          <button type="button" className="toolbar-brand topbar-brand topbar-brand-menu-trigger"
            aria-label={ui(t("app.openNavigationMenu"))} title={ui(t("app.menu"))}>
            IronSmith
          </button>
        ) : <Button
          variant="secondary"
          size="icon-xs"
          className="stone-pill topbar-menu-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]"
          aria-label={ui(triggerIcon === "menu" ? t("app.openNavigationMenu") : t("app.openGameMenu"))}
          title={ui(triggerIcon === "menu" ? t("app.menu") : t("app.settings"))}
        >
          {triggerGlyph}
        </Button>}
      </SheetTrigger>
      <SheetContent
        side={nonDesktopViewport ? "center" : "right"}
        className="fantasy-sheet settings-sheet w-[min(94vw,420px)] overflow-y-auto p-0"
      >
        <SheetHeader className="fantasy-sheet-header pr-12">
          <div className="text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{t("app.menu")}</div>
          <SheetTitle>
            {t("settings.table")}
          </SheetTitle>
          <SheetDescription className="max-w-[32ch] text-[13px] leading-5">
            {t("settings.description")}
          </SheetDescription>
        </SheetHeader>

        <Tabs value={tab} onValueChange={selectTab} className="settings-sheet-tabs gap-0">
          <TabsList variant="line" className="settings-sheet-tablist" aria-label={ui("Menu sections")}>
            {MENU_TABS.map((entry) => {
              const TabIcon = entry.Icon;
              return (
                <TabsTrigger key={entry.value} value={entry.value} className="settings-sheet-tab">
                  <TabIcon className="size-3.5" aria-hidden="true" />
                  {ui(entry.label)}
                </TabsTrigger>
              );
            })}
          </TabsList>

          <TabsContent value="play" className="settings-sheet-body grid px-4 pb-4">
            <MenuSection
              className="settings-quick-actions"
              eyebrow="Play"
              title={inMatch ? "Current match" : "Start a game"}
              description={inMatch
                ? "Concede the match in progress, or reopen its lobby."
                : "Host or join a multiplayer lobby, or set up a table to play on your own."}
            >
              <div className="grid gap-2 sm:grid-cols-2">
                {inMatch ? <SurrenderButton onClick={() => setOpen(false)} /> : null}
                <Button variant="secondary" size="sm" className="stone-pill justify-start" onClick={handleOpenLobby}>
                  <Users className="size-3.5" />
                  {lobbyBusy ? t("action.openLobby") : t("action.createLobby")}
                </Button>
                {onGenerateRandomGame ? (
                  <RandomGameSheet
                    onGenerate={onGenerateRandomGame}
                    disabled={lobbyBusy}
                    trigger={(
                      <Button variant="secondary" size="sm" className="stone-pill justify-start" disabled={lobbyBusy}>
                        <Shuffle className="size-3.5" />
                        {t("action.randomGame")}
                      </Button>
                    )}
                  />
                ) : null}
                <Button variant="secondary" size="sm" className="stone-pill justify-start" disabled={lobbyBusy} onClick={handleOpenPuzzleSetup}>
                  <Puzzle className="size-3.5" />
                  {puzzleSetupMode ? t("action.closePuzzle") : t("action.puzzleSetup")}
                </Button>
              </div>
            </MenuSection>
            <MenuSection
              eyebrow={t("settings.match.eyebrow")}
              title={t("settings.match.title")}
              description={t("settings.match.description")}
            >
              <div className="grid gap-3 sm:grid-cols-2">
                {playerNameSlots.map((playerName, index) => (
                  <label key={index} className={labelClass}>
                    {`${t("settings.player")} ${index + 1}`}
                    <span className="settings-seat-row">
                      <input
                        className={inputClass}
                        value={playerName}
                        placeholder={defaultSeatName(index)}
                        disabled={lobbyBusy}
                        onChange={(event) => updatePlayerName(index, event.target.value)}
                      />
                      {seatCount > MIN_TABLE_SEATS ? (
                        <button
                          type="button"
                          className="settings-seat-remove"
                          aria-label={ui("Remove {0}", { 0: playerName || defaultSeatName(index) })}
                          title={ui("Remove player")}
                          disabled={lobbyBusy}
                          onClick={(event) => { event.preventDefault(); removePlayer(index); }}
                        >
                          <X className="size-3.5" aria-hidden="true" />
                        </button>
                      ) : null}
                    </span>
                  </label>
                ))}
                {seatCount < MAX_TABLE_SEATS ? (
                  <Button
                    variant="secondary"
                    size="sm"
                    className="stone-pill settings-add-seat justify-start self-end"
                    disabled={lobbyBusy}
                    onClick={addPlayer}
                  >
                    <UserPlus className="size-3.5" />
                    {ui("Add player")}
                  </Button>
                ) : null}
                <label className={labelClass}>
                  {t("settings.startingLife")}
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
              <p className="m-0 text-[13px] leading-5 text-muted-foreground">
                {ui(commanderTable
                  ? "Reset Match deals random Commander or Pauper EDH decks to {0} players, at that format's starting life."
                  : "Reset Match deals two random decks from Modern, Legacy, Pioneer, Standard, Vintage or Pauper. Add a third player for Commander.",
                { 0: seatCount })}
              </p>
              <div className="grid gap-2">
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={fixedStartingBoard}
                    onCheckedChange={(value) => setFixedStartingBoard(value === true)}
                  />
                  {t("settings.fixedStartingBoard")}
                </label>
                <p className="m-0 text-[13px] leading-5 text-muted-foreground">
                  {t("settings.fixedStartingBoard.description")}
                </p>
              </div>
              <Button
                variant="destructive"
                size="sm"
                className="settings-reset-button"
                disabled={lobbyBusy}
                onClick={onReset}
              >
                {t("action.resetMatch")}
              </Button>
            </MenuSection>
          </TabsContent>

          <TabsContent value="table" className="settings-sheet-body grid px-4 pb-4">
            <MenuSection
              eyebrow="Table"
              title="This table"
              description="Share, check, or reload the game in front of you."
            >
              <div className="grid gap-2 sm:grid-cols-2">
                <Button variant="secondary" size="sm" className="stone-pill justify-start" onClick={handleShareCurrentTable}>
                  <Share2 className="size-3.5" />
                  {t("action.shareTable")}
                </Button>
                <VerifyMatchSheet
                  trigger={(
                    <Button
                      variant="secondary"
                      size="sm"
                      className="stone-pill justify-start"
                    >
                      <ShieldCheck className="size-3.5" />
                      {t("action.verifyMatch")}
                    </Button>
                  )}
                />
                <Button variant="secondary" size="sm" className="stone-pill justify-start" onClick={handleToggleLog}>
                  <ScrollText className="size-3.5" />
                  {t("settings.openLog")}
                </Button>
                <Button variant="secondary" size="sm" className="stone-pill justify-start" onClick={handleRefresh}>
                  <RefreshCw className="size-3.5" />
                  {t("action.refreshView")}
                </Button>
                <ExportMatchButton className="stone-pill inline-flex items-center justify-start gap-1.5 rounded-none px-2.5 py-2 text-[13px] font-medium uppercase hover:brightness-110" />
              </div>
              <AuditReplayControls />
            </MenuSection>
            <MenuSection
              eyebrow={t("settings.info.eyebrow")}
              title={t("settings.info.title")}
              description={t("settings.info.description")}
            >
              <div className="grid gap-2 text-[13px] text-foreground">
                <div className="fantasy-sheet-stat flex items-center justify-between gap-3 px-3 py-2">
                  <span className="uppercase tracking-[0.16em] text-muted-foreground">{t("settings.view")}</span>
                  <Badge variant="secondary" className="fantasy-sheet-badge text-[12px] uppercase">
                    {me?.name || "-"}
                  </Badge>
                </div>
                <div className="fantasy-sheet-stat flex items-center justify-between gap-3 px-3 py-2">
                  <span className="uppercase tracking-[0.16em] text-muted-foreground">{t("settings.lobby")}</span>
                  <Badge variant="secondary" className="fantasy-sheet-badge text-[12px] uppercase">
                    {ui(lobbyLabel)}
                  </Badge>
                </div>
                {multiplayer.matchStarted && offlinePlayers.length > 0 ? (
                  <div className="fantasy-sheet-stat flex items-center justify-between gap-3 bg-[#2b1114] px-3 py-2">
                    <span className="uppercase tracking-[0.16em] text-[#f0a9a0]">
                      {t("settings.disconnected")}
                    </span>
                    <Badge variant="secondary" className="fantasy-sheet-badge max-w-[180px] truncate text-[12px] uppercase text-[#f0a9a0]">
                      {offlinePlayers.map((player) => {
                        const display = playerDisplayName(
                          state?.players || [],
                          player.playerIndex ?? player.index ?? player.id
                        );
                        return display === "?" ? player.name : display;
                      }).join(", ")}
                    </Badge>
                  </div>
                ) : null}
              </div>
            </MenuSection>
          </TabsContent>

          <TabsContent value="settings" className="settings-sheet-body grid px-4 pb-4">
            <MenuSection
              eyebrow={t("settings.live.eyebrow")}
              title={t("settings.live.title")}
              description="How passing priority behaves, and who you play as in local games."
            >
              <div className="grid gap-2">
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={!!priorityStatus?.fullControl}
                    title={ui("Manually confirm every priority and choice.")}
                    onCheckedChange={(value) => priorityPolicy?.setFullControl(Boolean(value), state)}
                  />
                  {ui("Full control")}
                </label>
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={followActivePlayer}
                    disabled={Boolean(multiplayer?.matchStarted)}
                    title={ui("In local games, switch the view to whichever player has to act, so one person can play every seat.")}
                    onCheckedChange={(value) => setFollowActivePlayer(Boolean(value))}
                  />
                  {ui("Follow active player")}
                </label>
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={showShortcutsOnStart}
                    onCheckedChange={(value) => setShowShortcutsOnStart(Boolean(value))}
                  />
                  {ui("Show shortcuts at start")}
                </label>
              </div>
              <div className="flex flex-wrap gap-2">
                <Button variant="secondary" size="sm" className="stone-pill" onClick={() => {
                  setOpen(false);
                  openKeyboardShortcuts();
                }}>
                  <Keyboard className="size-3.5" />
                  {ui("Keyboard shortcuts")}
                </Button>
              </div>
            </MenuSection>
            <MenuSection
              eyebrow={t("settings.appearance.eyebrow")}
              title={t("settings.appearance.title")}
              description={t("settings.appearance.description")}
            >
              <label className={labelClass}>
                {ui("Graphics")}
                <select className={inputClass} aria-label={ui("Graphics")}
                  value={graphicsQuality} onChange={event => setGraphicsQuality(event.target.value)}>
                  <option value="off">{ui("Off")}</option>
                  <option value="low">{ui("Low")}</option>
                  <option value="high">{ui("High")}</option>
                </select>
                <span>{ui("Off: static board. Low: simple scenery. High: full effects. Card and text clarity stays the same.")}</span>
              </label>
              <label className={labelClass}>
                {ui("Battlefield cards")}
                <select className={inputClass} aria-label={ui("Battlefield cards")}
                  value={battlefieldAppearance.compactCards ? "compact" : "full"}
                  onChange={event => setBattlefieldAppearance("compactCards", event.target.value === "compact")}>
                  <option value="compact">{ui("Compact artwork")}</option>
                  <option value="full">{ui("Full cards")}</option>
                </select>
              </label>
              <label className={labelClass}>
                {ui("Noncreature lands")}
                <select className={inputClass} aria-label={ui("Noncreature lands")}
                  value={battlefieldAppearance.compactLands ? "compact" : "standard"}
                  onChange={event => setBattlefieldAppearance("compactLands", event.target.value === "compact")}>
                  <option value="compact">{ui("Small artwork tiles")}</option>
                  <option value="standard">{ui("Match battlefield cards")}</option>
                </select>
              </label>
              <label className={labelClass}>
                {ui("Card finishes")}
                <select className={inputClass} aria-label={ui("Card finishes")}
                  value={battlefieldAppearance.cardFinishEffects ? "live" : "static"}
                  onChange={event => setBattlefieldAppearance("cardFinishEffects", event.target.value === "live")}>
                  <option value="static">{ui("Static")}</option>
                  <option value="live">{ui("Live foil and print effects")}</option>
                </select>
                <span>{ui("Live effects animate custom card finishes in the inspector and use more graphics resources.")}</span>
              </label>
              <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto]">
                <label className={labelClass}>
                  {t("settings.interfaceFont")}
                  <select
                    className={inputClass}
                    value={uiFont}
                    onChange={(event) => setUiFont(event.target.value)}
                  >
                    {UI_FONT_OPTIONS.map((font) => (
                      <option key={font.name} value={font.name}>{font.name}</option>
                    ))}
                  </select>
                </label>
                <label className={`${labelClass} settings-accent-control`}>
                  {t("settings.playerAccent")}
                  <input
                    className="settings-accent-input"
                    type="color"
                    value={perspectiveAccent?.hex || "#b79cff"}
                    onChange={(event) => setPlayerAccentOverride(perspective ?? 0, event.target.value)}
                    aria-label={t("settings.playerAccent")}
                  />
                </label>
              </div>
            </MenuSection>
            <MenuSection
              eyebrow={t("settings.language.eyebrow")}
              title={t("settings.language.title")}
              description={t("settings.language.description")}
            >
              <label className={labelClass}>
                {t("settings.language.label")}
                <select
                  className={inputClass}
                  value={locale}
                  onChange={(event) => setLocale(event.target.value)}
                >
                  {locales.map((entry) => (
                    <option key={entry.id} value={entry.id}>
                      {ui(entry.label)}
                    </option>
                  ))}
                </select>
              </label>
            </MenuSection>
          </TabsContent>

          <TabsContent value="developer" className="settings-sheet-body grid px-4 pb-4">
            <MenuSection
              eyebrow="Developer"
              title="Cards"
              description="Put any card onto the table, build custom cards, or compile your own rules text."
            >
              {inMatch ? (
                <p className="m-0 text-[13px] leading-5 text-muted-foreground">
                  {ui("Card tools are unavailable during a multiplayer match.")}
                </p>
              ) : (
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
                          {t("action.addCard")}
                        </Button>
                      )}
                    />
                    <CustomCardsButton className="stone-pill justify-start px-3 py-2 text-sm disabled:opacity-45" />
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
                          {t("action.compileCard")}
                        </button>
                      )}
                    />
                </div>
              )}
            </MenuSection>
            <MenuSection
              eyebrow="Developer"
              title="Card pool"
              description="Cards are only offered when their compiled rules match the printed text at least this closely."
            >
              <div className="grid gap-2 text-[13px] text-foreground">
                <div className="fantasy-sheet-stat flex items-center justify-between gap-3 px-3 py-2">
                  <span className="uppercase tracking-[0.16em] text-muted-foreground">
                    {t("settings.cardsCompiled")}
                  </span>
                  <Badge variant="secondary" className="fantasy-sheet-badge text-[12px] uppercase">
                    {ui(compiledLabel)}
                  </Badge>
                </div>
              </div>
              <label className={`${labelClass} gap-2`}>
                <span className="flex items-center justify-between gap-3">
                  <span>{t("settings.fidelityThreshold")}</span>
                  <span className="settings-fidelity-value tabular-nums">
                    {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : t("fidelity.off")}
                    {` · ${cardsMeetingThreshold}`}
                  </span>
                </span>
                <Slider
                  aria-label={ui("Card fidelity threshold")}
                  min={0}
                  max={100}
                  step={1}
                  value={[Math.round(semanticThreshold)]}
                  onValueChange={([value]) => setSemanticThreshold(value)}
                />
              </label>
            </MenuSection>
            <MenuSection
              eyebrow="Developer"
              title="Debugging"
              description="Engine diagnostics and extra inspector detail."
            >
              <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                <Checkbox
                  checked={inspectorDebug}
                  onCheckedChange={(value) => setInspectorDebug(Boolean(value))}
                />
                {t("settings.debug")}
              </label>
              <div className="flex flex-wrap gap-2">
                <DiagnosticsSheet
                  trigger={(
                    <Button variant="secondary" size="sm" className="stone-pill justify-start">
                      <Activity className="size-3.5" />
                      {t("action.diagnostics")}
                    </Button>
                  )}
                />
              <Button variant="secondary" size="sm" className="stone-pill" asChild>
                <a
                  href="https://github.com/Chiplis/ironsmith"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <Github className="size-3.5" />
                  {t("settings.repository")}
                  <ExternalLink className="size-3" />
                </a>
              </Button>
              </div>
            </MenuSection>
          </TabsContent>
        </Tabs>

      </SheetContent>
    </Sheet>
  );
}
