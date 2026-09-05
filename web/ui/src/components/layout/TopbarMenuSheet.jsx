import { useEffect, useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
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
import { ExternalLink, Github, Menu, RefreshCw, Settings2, ShieldCheck } from "lucide-react";
import AddCardSheet from "./AddCardSheet";
import CreateCardForgeSheet from "./CreateCardForgeSheet";
import VerifyMatchSheet from "./VerifyMatchSheet";
import { playerDisplayName } from "@/lib/player-display";
import { getPlayerAccent } from "@/lib/player-colors";
import { UI_FONT_OPTIONS } from "@/lib/ui-fonts";
import { useI18n } from "@/i18n/I18nContext";

const inputClass =
  "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none disabled:cursor-not-allowed disabled:opacity-50";
const labelClass =
  "grid gap-1 text-[11px] uppercase tracking-[0.2em] text-muted-foreground";
const sectionClass =
  "fantasy-sheet-section settings-sheet-section grid gap-3 py-5";

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
  } = useGame();
  const { locale, locales, setLocale, t } = useI18n();

  const players = useMemo(() => state?.players || [], [state?.players]);
  const perspective = state?.perspective;
  const me = players.find((player) => player.id === perspective) || players[0];
  const perspectiveAccent = getPlayerAccent(
    players,
    perspective,
    perspective ?? 0,
    playerAccentOverrides
  );
  const playerNameSlots = useMemo(() => {
    const configuredNames = String(playerNames || "").split(",");
    const slotCount = Math.max(players.length, configuredNames.length, 2);
    return Array.from({ length: slotCount }, (_, index) => (
      configuredNames[index]?.trim()
      || players[index]?.name
      || `Player ${index + 1}`
    ));
  }, [playerNames, players]);
  const lobbyBusy = multiplayer.mode !== "idle";
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
    nextNames[index] = value;
    setPlayerNames(nextNames.join(","));
  };

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
          aria-label={triggerIcon === "menu" ? t("app.openNavigationMenu") : t("app.openGameMenu")}
          title={triggerIcon === "menu" ? t("app.menu") : t("app.settings")}
        >
          {triggerGlyph}
        </Button>
      </SheetTrigger>
      <SheetContent
        side={nonDesktopViewport ? "center" : "right"}
        className="fantasy-sheet settings-sheet w-[min(94vw,420px)] overflow-y-auto p-0"
      >
        <SheetHeader className="fantasy-sheet-header pr-12">
          <div className="text-[11px] uppercase tracking-[0.24em] text-[#cdb27a]">{t("app.menu")}</div>
          <SheetTitle className="text-[22px] uppercase tracking-[0.18em] text-foreground">
            {t("settings.table")}
          </SheetTitle>
          <SheetDescription className="max-w-[32ch] text-[13px] leading-5">
            {t("settings.description")}
          </SheetDescription>
        </SheetHeader>

        <div className="settings-sheet-body grid px-4 pb-4">
          {showQuickActions ? (
            <MenuSection
              eyebrow={t("settings.quick.eyebrow")}
              title={t("settings.quick.title")}
              description={t("settings.quick.description")}
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
                      {t("action.addCard")}
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
                      {t("action.compileCard")}
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
                  {deckLoadingMode ? t("action.cancelDeckLoad") : t("action.loadDecks")}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  disabled={lobbyBusy}
                  onClick={handleOpenPuzzleSetup}
                >
                  {puzzleSetupMode ? t("action.closePuzzle") : t("action.puzzleSetup")}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  onClick={handleShareCurrentTable}
                >
                  {t("action.shareTable")}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  onClick={handleOpenLobby}
                >
                  {lobbyBusy ? t("action.openLobby") : t("action.createLobby")}
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
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  onClick={handleToggleLog}
                >
                  {t("settings.openLog")}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  className="stone-pill justify-start"
                  onClick={handleRefresh}
                >
                  <RefreshCw className="size-3.5" />
                  {t("action.refreshView")}
                </Button>
              </div>
              <Button variant="secondary" size="sm" className="stone-pill justify-start" asChild>
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
            </MenuSection>
          ) : null}
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
                    {entry.label}
                  </option>
                ))}
              </select>
            </label>
          </MenuSection>

          <MenuSection
            eyebrow={t("settings.appearance.eyebrow")}
            title={t("settings.appearance.title")}
            description={t("settings.appearance.description")}
          >
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
            <label className={`${labelClass} gap-2`}>
              <span className="flex items-center justify-between gap-3">
                <span>{t("settings.fidelityThreshold")}</span>
                <span className="settings-fidelity-value tabular-nums">
                  {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : t("fidelity.off")}
                  {` · ${cardsMeetingThreshold}`}
                </span>
              </span>
              <Slider
                min={0}
                max={100}
                step={1}
                value={[Math.round(semanticThreshold)]}
                onValueChange={([value]) => setSemanticThreshold(value)}
              />
            </label>
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
                  <input
                    className={inputClass}
                    value={playerName}
                    disabled={lobbyBusy}
                    onChange={(event) => updatePlayerName(index, event.target.value)}
                  />
                </label>
              ))}
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
            <div className="grid gap-2 sm:grid-cols-2">
              <Button
                variant="destructive"
                size="sm"
                className="settings-reset-button"
                disabled={lobbyBusy}
                onClick={onReset}
              >
                {t("action.resetMatch")}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={lobbyBusy}
                onClick={handleToggleDeckLoading}
              >
                {deckLoadingMode ? t("action.cancelDeckLoad") : t("action.loadDecks")}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                className="stone-pill"
                disabled={lobbyBusy}
                onClick={handleOpenPuzzleSetup}
              >
                {puzzleSetupMode ? t("action.closePuzzle") : t("action.puzzleSetup")}
              </Button>
              <Button variant="secondary" size="sm" className="stone-pill" onClick={handleShareCurrentTable}>
                {t("action.shareTable")}
              </Button>
              <Button variant="secondary" size="sm" className="stone-pill" onClick={handleOpenLobby}>
                {lobbyBusy ? t("action.openLobby") : t("action.createLobby")}
              </Button>
              <Button variant="secondary" size="sm" className="stone-pill" onClick={handleRefresh}>
                <RefreshCw className="size-3.5" />
                {t("action.refreshView")}
              </Button>
            </div>
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
                <span className="uppercase tracking-[0.16em] text-muted-foreground">
                  {t("settings.cardsCompiled")}
                </span>
                <Badge variant="secondary" className="fantasy-sheet-badge text-[12px] uppercase">
                  {compiledLabel}
                </Badge>
              </div>
              <div className="fantasy-sheet-stat flex items-center justify-between gap-3 px-3 py-2">
                <span className="uppercase tracking-[0.16em] text-muted-foreground">{t("settings.lobby")}</span>
                <Badge variant="secondary" className="fantasy-sheet-badge text-[12px] uppercase">
                  {lobbyLabel}
                </Badge>
              </div>
              {multiplayer.matchStarted && offlinePlayers.length > 0 ? (
                <div className="fantasy-sheet-stat flex items-center justify-between gap-3 border-[#7d302f] bg-[#2b1114]/60 px-3 py-2">
                  <span className="uppercase tracking-[0.16em] text-[#ffb8c0]">
                    {t("settings.disconnected")}
                  </span>
                  <Badge variant="secondary" className="fantasy-sheet-badge max-w-[180px] truncate text-[12px] uppercase text-[#ffb8c0]">
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
          </MenuSection>

          <MenuSection
            eyebrow={t("settings.live.eyebrow")}
            title={t("settings.live.title")}
            description={t("settings.live.description")}
          >
            <div className="grid gap-3 sm:grid-cols-2">
              <label className={labelClass}>
                {t("settings.autoPassHold")}
                <select
                  className={inputClass}
                  value={holdRule}
                  onChange={(event) => setHoldRule(event.target.value)}
                >
                  <option value="never">{t("hold.never")}</option>
                  <option value="if_actions">{t("hold.ifActions")}</option>
                  <option value="stack">{t("hold.stack")}</option>
                  <option value="main">{t("hold.main")}</option>
                  <option value="combat">{t("hold.combat")}</option>
                  <option value="ending">{t("hold.ending")}</option>
                  <option value="always">{t("hold.always")}</option>
                </select>
              </label>
              <div className="grid gap-2">
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={autoPassEnabled}
                    onCheckedChange={(value) => setAutoPassEnabled(Boolean(value))}
                  />
                  {t("action.autoPass")}
                </label>
                <label className="flex items-center gap-2 text-[13px] uppercase tracking-[0.14em] text-muted-foreground">
                  <Checkbox
                    checked={inspectorDebug}
                    onCheckedChange={(value) => setInspectorDebug(Boolean(value))}
                  />
                  {t("settings.debug")}
                </label>
              </div>
            </div>
            <Button variant="secondary" size="sm" className="stone-pill" onClick={handleToggleLog}>
              {t("settings.openLog")}
            </Button>
          </MenuSection>
        </div>
      </SheetContent>
    </Sheet>
  );
}
