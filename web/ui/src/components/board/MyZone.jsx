import { useCallback } from "react";
import { useGame } from "@/context/GameContext";
import BattlefieldRow from "./BattlefieldRow";
import DeckZonePile from "./DeckZonePile";
import ManaPool from "@/components/left-rail/ManaPool";
import StackTimelineRail from "@/components/right-rail/StackTimelineRail";
import { getPlayerAccent } from "@/lib/player-colors";
import { cn } from "@/lib/utils";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";

const ZONE_ORDER = ["battlefield", "hand", "graveyard", "library", "exile", "command"];
const SLIDE_IN_ZONE_IDS = new Set(["graveyard", "exile"]);
const ZONE_LABELS = {
  battlefield: "Battlefield",
  hand: "Hand",
  graveyard: "GY",
  library: "Deck",
  exile: "Exile",
  command: "CZ",
};
const MY_ZONE_HEADER_HEIGHT = 44;

function normalizeZoneViews(zoneViews) {
  const normalized = Array.isArray(zoneViews)
    ? zoneViews.filter((zone) => ZONE_ORDER.includes(zone))
    : [];
  return Array.from(new Set(["battlefield", ...normalized]));
}

function getZoneCards(player, zone) {
  switch (zone) {
    case "hand": return player.hand_cards || [];
    case "graveyard": return player.graveyard_cards || [];
    case "library": return [];
    case "exile": return player.exile_cards || [];
    case "command": return player.command_cards || [];
    default: return player.battlefield || [];
  }
}

function getZoneCount(player, zone) {
  switch (zone) {
    case "hand":
      return player.hand_size ?? 0;
    case "graveyard":
      return player.graveyard_size ?? 0;
    case "library":
      return player.library_size ?? 0;
    case "exile":
      return Array.isArray(player.exile_cards) ? player.exile_cards.length : 0;
    case "command":
      return player.command_size ?? (Array.isArray(player.command_cards) ? player.command_cards.length : 0);
    default:
      return (player.battlefield || []).reduce((total, card) => {
        const count = Number(card.count);
        return total + (Number.isFinite(count) && count > 1 ? count : 1);
      }, 0);
  }
}

function buildZoneEntries(player, zoneViews) {
  const activeZones = normalizeZoneViews(zoneViews);
  return ZONE_ORDER.map((zone) => ({
    zone,
    label: ZONE_LABELS[zone] || zone,
    cards: getZoneCards(player, zone),
    count: getZoneCount(player, zone),
    active: activeZones.includes(zone),
  }));
}

function zoneCounts(player) {
  const exileCards = Array.isArray(player.exile_cards) ? player.exile_cards : [];
  const commandCards = Array.isArray(player.command_cards) ? player.command_cards : [];
  const battlefieldCount = (player.battlefield || []).reduce((total, card) => {
    const count = Number(card.count);
    return total + (Number.isFinite(count) && count > 1 ? count : 1);
  }, 0);

  return [
    { label: "Battlefield", count: battlefieldCount },
    { label: "Hand", count: player.hand_size ?? 0 },
    { label: "GY", count: player.graveyard_size ?? 0 },
    { label: "Deck", count: player.library_size ?? 0 },
    { label: "Exile", count: exileCards.length },
    { label: "CZ", count: player.command_size ?? commandCards.length },
  ];
}

function isBaseVisibleZone(zone, zoneViews, count) {
  const baseViews = normalizeZoneViews(zoneViews);
  if (!baseViews.includes(zone)) return false;
  return zone === "battlefield" || zone === "library" || count > 0;
}

function formatZoneActivityClass(direction) {
  return direction === "left"
    ? "zone-auto-reveal zone-auto-reveal-leave"
    : "zone-auto-reveal zone-auto-reveal-enter";
}

function collectCardObjectIds(card) {
  const ids = [Number(card?.id)];
  if (Array.isArray(card?.member_ids)) {
    for (const memberId of card.member_ids) {
      ids.push(Number(memberId));
    }
  }
  return ids.filter((id) => Number.isFinite(id));
}

function ZoneCountInline({ player }) {
  const counts = zoneCounts(player);
  return (
    <div className="battlefield-counts flex items-center gap-2 text-[11px] uppercase tracking-wide text-[#8ea8c8] whitespace-nowrap">
      {counts.map((entry) => (
        <span key={entry.label}>
          <span className="font-bold text-[#c1d4ea]">{entry.label}</span>{" "}
          <span className="text-[#d6e6fb] font-semibold">{entry.count}</span>
        </span>
      ))}
    </div>
  );
}

export default function MyZone({
  player,
  selectedObjectId,
  onInspect,
  zoneViews = ["battlefield"],
  zoneActivity = {},
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
}) {
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const { state } = useGame();
  const playerAccent = getPlayerAccent(state?.players || [], player?.id);

  const transientZoneViews = Object.keys(zoneActivity || {});
  const zoneEntries = buildZoneEntries(player, [...zoneViews, ...transientZoneViews]);
  const activeZoneEntries = zoneEntries.filter((entry) => entry.active);
  const boardZoneEntries = activeZoneEntries.filter((entry) => !SLIDE_IN_ZONE_IDS.has(entry.zone));
  const overlayZoneEntries = activeZoneEntries.filter((entry) => SLIDE_IN_ZONE_IDS.has(entry.zone));
  const visibleZones = new Set(
    boardZoneEntries
      .filter((entry) =>
        entry.zone === "battlefield"
        || entry.zone === "library"
        || entry.count > 0
        || Boolean(zoneActivity?.[entry.zone])
      )
      .map((entry) => entry.zone)
  );
  if (visibleZones.size === 0 && boardZoneEntries.length > 0) {
    visibleZones.add(boardZoneEntries[0].zone);
  }
  const zoneName = boardZoneEntries.length === 1
    ? (boardZoneEntries[0].zone === "battlefield" ? "" : ` — ${boardZoneEntries[0].label}`)
    : "";
  const showZoneHeaders = visibleZones.size > 1;
  const isActivePlayer = Number(state?.active_player) === Number(player?.id);
  const isPriorityPlayer = Number(state?.priority_player) === Number(player?.id);
  const isPlayerLegalTarget =
    legalTargetPlayerIds.has(Number(player.id)) || legalTargetPlayerIds.has(Number(player.index));
  const canPickTargetFromBoard = state?.decision?.kind === "targets"
    && state?.decision?.player === state?.perspective;

  // Build activatable map from decision actions (activate_ability + activate_mana_ability)
  const activatableMap = new Map();
  if (state?.decision?.kind === "priority" && state.decision.actions) {
    for (const action of state.decision.actions) {
      if (
        (action.kind === "activate_ability" || action.kind === "activate_mana_ability") &&
        action.object_id != null
      ) {
        const objId = Number(action.object_id);
        if (!activatableMap.has(objId)) activatableMap.set(objId, []);
        activatableMap.get(objId).push(action);
      }
    }
  }

  const handleCardClick = (_e, card) => {
    if (canPickTargetFromBoard && !shouldHandleClick(_e)) return;
    const candidateObjectIds = collectCardObjectIds(card);

    if (canPickTargetFromBoard) {
      const matchedTargetId = candidateObjectIds.find((id) => legalTargetObjectIds.has(id));
      if (matchedTargetId != null) {
        window.dispatchEvent(
          new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "object", object: matchedTargetId } },
          })
        );
        return;
      }
    }

    onInspect?.(card.id, { candidateObjectIds });
  };

  const handleCardPointerDown = useCallback((event, card) => {
    if (!canPickTargetFromBoard || !registerPointerDown(event)) return;
    const candidateObjectIds = collectCardObjectIds(card);
    const matchedTargetId = candidateObjectIds.find((id) => legalTargetObjectIds.has(id));
    if (matchedTargetId == null) return;
    event.preventDefault();
    event.stopPropagation();
    window.dispatchEvent(
      new CustomEvent("ironsmith:target-choice", {
        detail: { target: { kind: "object", object: matchedTargetId } },
      })
    );
  }, [canPickTargetFromBoard, legalTargetObjectIds, registerPointerDown]);

  const dispatchPlayerTargetChoice = useCallback(() => {
    if (!canPickTargetFromBoard || !isPlayerLegalTarget) return;
    const targetPlayer = legalTargetPlayerIds.has(Number(player.id))
      ? Number(player.id)
      : Number(player.index);
    if (!Number.isFinite(targetPlayer)) return;
    window.dispatchEvent(
      new CustomEvent("ironsmith:target-choice", {
        detail: { target: { kind: "player", player: targetPlayer } },
      })
    );
  }, [
    canPickTargetFromBoard,
    isPlayerLegalTarget,
    legalTargetPlayerIds,
    player.id,
    player.index,
  ]);

  const handlePlayerTargetPointerDown = useCallback((event) => {
    if (!registerPointerDown(event)) return;
    event.preventDefault();
    event.stopPropagation();
    dispatchPlayerTargetChoice();
  }, [dispatchPlayerTargetChoice, registerPointerDown]);

  const handlePlayerTargetClick = useCallback((event) => {
    if (!shouldHandleClick(event)) return;
    event.preventDefault();
    event.stopPropagation();
    dispatchPlayerTargetChoice();
  }, [dispatchPlayerTargetChoice, shouldHandleClick]);

  return (
    <section
      className="board-zone-bg battlefield-panel battlefield-panel--self relative z-[28] min-h-0 h-full overflow-visible grid p-0"
      style={{
        gridTemplateRows: `${MY_ZONE_HEADER_HEIGHT}px minmax(0,1fr)`,
        alignContent: "stretch",
        "--player-accent": playerAccent?.hex || "#d8bf6a",
        "--panel-accent": playerAccent?.hex || "#b98946",
        "--player-accent-rgb": playerAccent?.rgb || "216, 191, 106",
      }}
      data-my-zone
    >
      <div className="relative min-h-0 overflow-visible">
        <div
          className="battlefield-panel-header relative z-[1] flex h-full items-center gap-2 overflow-visible pr-2"
          data-my-zone-header-content
          data-turn-priority={isPriorityPlayer ? "true" : "false"}
        >
          <span
            className={cn(
              "battlefield-life text-[23px] font-bold leading-none text-[#f5d08b] tabular-nums",
              isPlayerLegalTarget
                && "text-[#d7ebff] rounded-none px-1 py-0.5 shadow-[0_0_10px_rgba(100,169,255,0.5)] ring-1 ring-[#64a9ff]/55"
            )}
            onPointerDown={handlePlayerTargetPointerDown}
            onClick={handlePlayerTargetClick}
            style={{ cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined }}
          >
            {player.life}
          </span>
          <span
            className={cn(
              "battlefield-name text-[16px] uppercase tracking-wider font-bold",
              isPlayerLegalTarget && "drop-shadow-[0_0_7px_rgba(100,169,255,0.7)]"
            )}
            data-player-target={player.id}
            data-player-target-name={player.id}
            onPointerDown={handlePlayerTargetPointerDown}
            onClick={handlePlayerTargetClick}
            style={{
              color: playerAccent?.hex,
              cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined,
            }}
          >
            <span className={cn(isActivePlayer && "battlefield-name-text--active")}>
              {player.name}
            </span>
            {zoneName && <span className="text-muted-foreground">{zoneName}</span>}
          </span>
          <ZoneCountInline player={player} />
          <ManaPool pool={player.mana_pool} />
        </div>
        <StackTimelineRail
          selectedObjectId={selectedObjectId}
          onInspectObject={onInspect}
        />
      </div>
      <div
        className="battlefield-zones-shell relative min-h-0 h-full overflow-visible"
        data-turn-active={isActivePlayer ? "true" : "false"}
      >
        {overlayZoneEntries.length > 0 ? (
          <div className="battlefield-overlay-zones pointer-events-none absolute inset-x-2 top-2 z-[4] flex justify-end gap-3">
            {overlayZoneEntries.map((entry) => {
              const activity = zoneActivity?.[entry.zone] || null;
              const displayCards = Array.isArray(activity?.replayCards) && activity.replayCards.length > 0
                ? activity.replayCards
                : entry.cards;
              const displayCount = Number.isFinite(activity?.displayCount) ? activity.displayCount : entry.count;
              return (
                <div
                  key={entry.zone}
                  className={cn(
                    "battlefield-overlay-zone pointer-events-auto",
                    activity && formatZoneActivityClass(activity.direction)
                  )}
                >
                  <div className="battlefield-overlay-zone-label flex items-center gap-2">
                    <span>{entry.label}</span>
                    <span className="text-[#f1e2c0]">{displayCount}</span>
                    {activity ? (
                      <span
                        className={cn(
                          "zone-activity-badge ml-auto",
                          activity.direction === "left"
                            ? "zone-activity-badge-leave"
                            : "zone-activity-badge-enter"
                        )}
                      >
                        {activity.label}
                      </span>
                    ) : null}
                  </div>
                  <div className="battlefield-overlay-zone-body min-h-0">
                    <BattlefieldRow
                      cards={displayCards}
                      compact
                      battlefieldSide="bottom"
                      selectedObjectId={selectedObjectId}
                      onCardClick={handleCardClick}
                      onCardPointerDown={handleCardPointerDown}
                      activatableMap={activatableMap}
                      legalTargetObjectIds={legalTargetObjectIds}
                      allowVerticalScroll
                      forceSingleColumn
                    />
                  </div>
                </div>
              );
            })}
          </div>
        ) : null}
        <div className="battlefield-zone-strip flex gap-1 min-h-0 h-full overflow-visible">
        {boardZoneEntries.map((entry) => {
          const isVisible = entry.active && visibleZones.has(entry.zone);
          const isPrimaryBattlefield = entry.zone === "battlefield";
          const isCompactSideZone = entry.zone === "library" || entry.zone === "command";
          const activity = zoneActivity?.[entry.zone] || null;
          const isTransientReveal = Boolean(activity)
            && !isBaseVisibleZone(entry.zone, zoneViews, entry.count);
          const displayCards = Array.isArray(activity?.replayCards) && activity.replayCards.length > 0
            ? activity.replayCards
            : entry.cards;
          const displayCount = Number.isFinite(activity?.displayCount) ? activity.displayCount : entry.count;
          return (
            <div
              key={entry.zone}
              data-zone-id={entry.zone}
              className={cn(
                "battlefield-zone-entry min-h-0 h-full",
                activity && formatZoneActivityClass(activity.direction)
              )}
              style={{
                flexGrow: isVisible ? (isPrimaryBattlefield ? 1 : 0) : 0,
                flexShrink: isPrimaryBattlefield ? 1 : 0,
                flexBasis: isVisible ? (
                  isPrimaryBattlefield
                    ? "0%"
                    : isCompactSideZone
                      ? "220px"
                      : "260px"
                ) : "0%",
                minWidth: isVisible ? "0px" : "0px",
                maxWidth: isVisible ? (isPrimaryBattlefield ? "100%" : (isCompactSideZone ? "240px" : "320px")) : "0px",
                opacity: isVisible ? 1 : 0,
                transform: isVisible ? "translateY(0)" : "translateY(4px)",
                pointerEvents: isVisible ? "auto" : "none",
                overflow: isVisible ? "visible" : "hidden",
                transition: isTransientReveal
                  ? "opacity 180ms ease, transform 220ms ease"
                  : "flex-grow 220ms ease, max-width 220ms ease, opacity 180ms ease, transform 220ms ease",
              }}
            >
              <div
                className={cn(
                  "grid gap-1 min-h-0 h-full",
                  isTransientReveal && "zone-reveal-content zone-reveal-content-enter"
                )}
                style={{ gridTemplateRows: showZoneHeaders || activity ? "auto minmax(0,1fr)" : "minmax(0,1fr)" }}
              >
                {(showZoneHeaders || activity) && (
                  <div className="battlefield-zone-label flex items-center gap-1 text-[11px] uppercase tracking-wide text-[#9cb8d8] px-0.5">
                    <span>{entry.label}</span>
                    <span className="text-[#d6e6fb]">{displayCount}</span>
                    {activity ? (
                      <span
                        className={cn(
                          "zone-activity-badge ml-auto",
                          activity.direction === "left"
                            ? "zone-activity-badge-leave"
                            : "zone-activity-badge-enter"
                        )}
                      >
                        {activity.label}
                      </span>
                    ) : null}
                  </div>
                )}
                {entry.zone === "library" ? (
                  <DeckZonePile count={displayCount} />
                ) : (
                  <BattlefieldRow
                    cards={displayCards}
                    compact={entry.zone !== "battlefield"}
                    battlefieldSide="bottom"
                    selectedObjectId={selectedObjectId}
                    onCardClick={handleCardClick}
                    onCardPointerDown={handleCardPointerDown}
                    activatableMap={activatableMap}
                    legalTargetObjectIds={legalTargetObjectIds}
                    allowVerticalScroll={entry.zone === "hand"}
                  />
                )}
              </div>
            </div>
          );
        })}
        </div>
      </div>

    </section>
  );
}
