import { useGame } from "@/context/GameContext";
import { formatPhase, formatStep } from "@/lib/constants";
import { Checkbox } from "@/components/ui/checkbox";
import { Slider } from "@/components/ui/slider";
import ZoneViewer from "@/components/board/ZoneViewer";
import { ComicTooltip } from "@/components/ui/comic-tooltip";
import { UI_FONT_OPTIONS } from "@/lib/ui-fonts";
import { getPlayerAccent } from "@/lib/player-colors";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";
import { useI18n } from "@/i18n/I18nContext";

const selectPill = "stone-select rounded-none px-2.5 py-0.5 text-[13px] font-medium border-0 outline-none cursor-pointer uppercase tracking-wide";
const fontListId = "ironsmith-ui-font-options";

export default function AddCardBar({
  compact = false,
  zoneViews = ["battlefield"],
  setZoneViews,
  onChangePerspective,
}) {
  const {
    state,
    semanticThreshold,
    setSemanticThreshold,
    cardsMeetingThreshold,
    multiplayer,
    autoPassEnabled,
    setAutoPassEnabled,
    holdRule,
    setHoldRule,
    uiFont,
    setUiFont,
    playerAccentOverrides,
    setPlayerAccentOverride,
  } = useGame();
  const { t } = useI18n();

  const players = state?.players || [];
  const perspective = state?.perspective ?? 0;
  const matchLocked = multiplayer.matchStarted;
  const activePlayer = players.find((player) => samePlayerId(player.id, state?.active_player)) || null;
  const priorityPlayer = players.find((player) => samePlayerId(player.id, state?.priority_player)) || null;
  const decisionPlayer = state?.decision?.player != null
    ? players.find((player) => samePlayerId(player.id, state.decision.player)) || null
    : null;
  const decisionOwnerDiffersFromPriority = decisionPlayer
    && (!priorityPlayer || !samePlayerId(decisionPlayer.id, priorityPlayer.id));
  const perspectiveAccent = getPlayerAccent(players, perspective, perspective, playerAccentOverrides);
  const translatedPhaseSummary = `${formatPhase(state?.phase, t)}${state?.step ? ` - ${formatStep(state?.step, t)}` : ""}`;

  return (
    <div className={`add-card-toolbar table-toolbar table-toolbar--secondary rounded-none px-3 py-2${compact ? " add-card-toolbar--compact" : ""}`}>
      <div className="add-card-toolbar-zone-group">
        <ZoneViewer zoneViews={zoneViews} setZoneViews={setZoneViews} embedded />
      </div>

      {!compact ? (
        <>
          <span className="add-card-toolbar-separator add-card-toolbar-fidelity-separator" aria-hidden="true" />
          <div className="add-card-toolbar-fidelity-group">
            <ComicTooltip
              title={t("fidelity.title")}
              description={t("fidelity.description")}
              side="top"
              align="start"
              sideOffset={7}
              contentClassName="max-w-[300px]"
            >
              <button
                type="button"
                className="add-card-toolbar-meta add-card-toolbar-help-trigger text-[13px] uppercase whitespace-nowrap"
                aria-label={t("fidelity.title")}
              >
                {t("fidelity.title")}
              </button>
            </ComicTooltip>
            <Slider
              className="w-20"
              min={0}
              max={100}
              step={1}
              value={[Math.round(semanticThreshold)]}
              onValueChange={([value]) => setSemanticThreshold(value)}
            />
            <span className="add-card-toolbar-meta-value text-[13px] tabular-nums whitespace-nowrap">
              {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : t("fidelity.off")}
              {" "}({cardsMeetingThreshold})
            </span>
          </div>
        </>
      ) : null}

      <span className="add-card-toolbar-separator add-card-toolbar-control-separator" aria-hidden="true" />

      <div className="add-card-toolbar-control-group">
        <select
          className={selectPill}
          value={holdRule}
          onChange={(event) => setHoldRule(event.target.value)}
          aria-label={t("settings.autoPassHold")}
        >
          <option value="never">{t("hold.never")}</option>
          <option value="if_actions">{t("hold.ifActions")}</option>
          <option value="stack">{t("hold.stack")}</option>
          <option value="main">{t("hold.main")}</option>
          <option value="combat">{t("hold.combat")}</option>
          <option value="ending">{t("hold.ending")}</option>
          <option value="always">{t("hold.always")}</option>
        </select>
        <label className="toolbar-checkbox add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap cursor-pointer uppercase">
          <Checkbox
            checked={autoPassEnabled}
            onCheckedChange={(value) => setAutoPassEnabled(!!value)}
            className="h-3.5 w-3.5"
          />
          {t("action.autoPass")}
        </label>
        <label className="add-card-toolbar-perspective add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap uppercase">
          <span>{t("action.playingAs")}</span>
          <select
            className={`${selectPill} add-card-toolbar-perspective-select`}
            value={perspective}
            disabled={matchLocked}
            onChange={(event) => onChangePerspective?.(Number(event.target.value))}
            aria-label={t("action.playingAs")}
          >
            {players.map((player) => (
              <option key={player.id} value={player.id}>
                {playerDisplayName(players, player)}
              </option>
            ))}
          </select>
        </label>
        <label className="add-card-toolbar-font add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap uppercase">
          <span>{t("action.font")}</span>
          <input
            className={`${selectPill} add-card-toolbar-font-input`}
            list={fontListId}
            value={uiFont}
            onChange={(event) => setUiFont(event.target.value)}
            aria-label={t("action.font")}
            spellCheck={false}
          />
          <datalist id={fontListId}>
            {UI_FONT_OPTIONS.map((font) => (
              <option key={font.name} value={font.name} />
            ))}
          </datalist>
        </label>
        <div className="add-card-toolbar-accent add-card-toolbar-toggle flex items-center gap-1.5 whitespace-nowrap uppercase">
          <span>{t("action.accent")}</span>
          <div
            className="add-card-toolbar-accent-swatch"
            style={{
              "--toolbar-player-accent": perspectiveAccent?.hex || "#731bde",
            }}
          >
            <input
              className="add-card-toolbar-accent-input add-card-toolbar-accent-input--player"
              type="color"
              value={perspectiveAccent?.hex || "#731bde"}
              onChange={(event) => setPlayerAccentOverride(perspective, event.target.value)}
              aria-label={t("action.accent")}
              title={t("action.accent")}
            />
          </div>
        </div>
      </div>

      <span className="add-card-toolbar-separator add-card-toolbar-phase-separator" aria-hidden="true" />

      <div className="topbar-phase-caption add-card-toolbar-phase-caption add-card-toolbar-phase-caption--trailing" aria-label={t("game.currentTurnSummary")}>
        <span>{translatedPhaseSummary}</span>
        <span className="topbar-phase-caption-dot" aria-hidden="true">•</span>
        <span>{t("game.turn", { turn: state?.turn_number ?? "-" })}</span>
        {activePlayer ? (
          <>
            <span className="topbar-phase-caption-dot" aria-hidden="true">•</span>
            <span>{t("game.activePlayer", { player: playerDisplayName(players, activePlayer) })}</span>
          </>
        ) : null}
        {decisionOwnerDiffersFromPriority ? (
          <>
            <span className="topbar-phase-caption-dot" aria-hidden="true">•</span>
            <span>{t("game.decisionPlayer", { player: playerDisplayName(players, decisionPlayer) })}</span>
          </>
        ) : priorityPlayer ? (
          <>
            <span className="topbar-phase-caption-dot" aria-hidden="true">•</span>
            <span>{t("game.priorityPlayer", { player: playerDisplayName(players, priorityPlayer) })}</span>
          </>
        ) : null}
      </div>
    </div>
  );
}
