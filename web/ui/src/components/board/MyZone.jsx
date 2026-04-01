import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import BattlefieldRow from "./BattlefieldRow";
import DeckZonePile from "./DeckZonePile";
import HandZone from "./HandZone";
import HoverArtOverlay from "@/components/right-rail/HoverArtOverlay";
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

function buildActivatableMap(decision) {
  const activatableMap = new Map();
  if (decision?.kind !== "priority" || !Array.isArray(decision.actions)) {
    return activatableMap;
  }

  for (const action of decision.actions) {
    if (
      (action.kind === "activate_ability" || action.kind === "activate_mana_ability")
      && action.object_id != null
    ) {
      const objId = Number(action.object_id);
      if (!activatableMap.has(objId)) activatableMap.set(objId, []);
      activatableMap.get(objId).push(action);
    }
  }

  return activatableMap;
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
  headerControls = null,
  embeddedActionBar = null,
  mobileBattleScene = false,
  playerAccent: explicitPlayerAccent = null,
  onMobileCardActionMenu = null,
  onMobileCardLongPress = null,
}) {
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const { state } = useGame();
  const mobileZoneRef = useRef(null);
  const mobileHandRef = useRef(null);
  const mobileHandMeasureRafRef = useRef(null);
  const [mobileHandViewportBounds, setMobileHandViewportBounds] = useState(null);
  const [mobileHandOcclusionViewportTop, setMobileHandOcclusionViewportTop] = useState(null);
  const playerAccent = explicitPlayerAccent || getPlayerAccent(state?.players || [], player?.id);
  const mergedMobileHeader = Boolean(embeddedActionBar);
  const mobileInspectorVisible = mergedMobileHeader && selectedObjectId != null;

  const transientZoneViews = Object.keys(zoneActivity || {});
  const zoneEntries = buildZoneEntries(player, [...zoneViews, ...transientZoneViews]);
  const activeZoneEntries = zoneEntries.filter((entry) => entry.active);
  const mobileHandRailVisible = mergedMobileHeader && Boolean(player?.can_view_hand);
  const boardZoneEntries = activeZoneEntries.filter((entry) => !SLIDE_IN_ZONE_IDS.has(entry.zone) && (!mobileHandRailVisible || entry.zone !== "hand"));
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
  const activatableMap = buildActivatableMap(state?.decision);

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

  const measureMobileHandOcclusion = useCallback(() => {
    const handEl = mobileHandRef.current;
    if (!handEl) {
      setMobileHandViewportBounds(null);
      setMobileHandOcclusionViewportTop(null);
      return;
    }

    const handRect = handEl.getBoundingClientRect();
    let nextTop = handRect.top;
    let nextLeft = handRect.left;
    let nextRight = handRect.right;
    let nextBottom = handRect.bottom;
    const stableCards = Array.from(
      handEl.querySelectorAll(".game-card.hand-card:not(.hovered):not(.inspected)")
    );
    const measuredCards = stableCards.length > 0
      ? stableCards
      : Array.from(handEl.querySelectorAll(".game-card.hand-card"));

    for (const cardEl of measuredCards) {
      const rect = cardEl.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) continue;
      nextTop = Math.min(nextTop, rect.top);
      nextLeft = Math.min(nextLeft, rect.left);
      nextRight = Math.max(nextRight, rect.right);
      nextBottom = Math.max(nextBottom, rect.bottom);
    }

    setMobileHandViewportBounds((currentBounds) => {
      if (
        currentBounds
        && Math.abs(currentBounds.top - nextTop) < 1
        && Math.abs(currentBounds.left - nextLeft) < 1
        && Math.abs(currentBounds.right - nextRight) < 1
        && Math.abs(currentBounds.bottom - nextBottom) < 1
      ) {
        return currentBounds;
      }
      return {
        top: nextTop,
        left: nextLeft,
        right: nextRight,
        bottom: nextBottom,
      };
    });
    setMobileHandOcclusionViewportTop((currentTop) => (
      currentTop != null && Math.abs(currentTop - nextTop) < 1
        ? currentTop
        : nextTop
    ));
  }, []);

  const scheduleMobileHandMeasure = useCallback(() => {
    if (mobileHandMeasureRafRef.current != null) return;
    mobileHandMeasureRafRef.current = window.requestAnimationFrame(() => {
      mobileHandMeasureRafRef.current = null;
      measureMobileHandOcclusion();
    });
  }, [measureMobileHandOcclusion]);

  useLayoutEffect(() => {
    if (!mobileBattleScene) return undefined;
    scheduleMobileHandMeasure();
    return undefined;
  }, [
    mobileBattleScene,
    player?.hand_size,
    player?.hand_cards?.length,
    selectedObjectId,
    scheduleMobileHandMeasure,
    state?.snapshot_id,
  ]);

  useEffect(() => {
    if (!mobileBattleScene) return undefined;

    const zoneEl = mobileZoneRef.current;
    const handEl = mobileHandRef.current;
    if (!zoneEl || !handEl) return undefined;

    const observer = new ResizeObserver(() => {
      scheduleMobileHandMeasure();
    });
    observer.observe(zoneEl);
    observer.observe(handEl);
    const onResize = () => scheduleMobileHandMeasure();
    window.addEventListener("resize", onResize);

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", onResize);
    };
  }, [mobileBattleScene, scheduleMobileHandMeasure]);

  useEffect(() => {
    if (typeof document === "undefined") return undefined;
    const root = document.documentElement;
    if (!mobileBattleScene || !mobileHandViewportBounds) {
      root.style.removeProperty("--mobile-battle-hand-top");
      root.style.removeProperty("--mobile-battle-hand-left");
      root.style.removeProperty("--mobile-battle-hand-right");
      root.style.removeProperty("--mobile-battle-hand-bottom");
      return undefined;
    }

    root.style.setProperty("--mobile-battle-hand-top", `${mobileHandViewportBounds.top}px`);
    root.style.setProperty("--mobile-battle-hand-left", `${mobileHandViewportBounds.left}px`);
    root.style.setProperty("--mobile-battle-hand-right", `${mobileHandViewportBounds.right}px`);
    root.style.setProperty("--mobile-battle-hand-bottom", `${mobileHandViewportBounds.bottom}px`);
    window.dispatchEvent(new CustomEvent("ironsmith:mobile-hand-bounds-change"));

    return () => {
      root.style.removeProperty("--mobile-battle-hand-top");
      root.style.removeProperty("--mobile-battle-hand-left");
      root.style.removeProperty("--mobile-battle-hand-right");
      root.style.removeProperty("--mobile-battle-hand-bottom");
      window.dispatchEvent(new CustomEvent("ironsmith:mobile-hand-bounds-change"));
    };
  }, [mobileBattleScene, mobileHandViewportBounds]);

  useEffect(() => () => {
    if (mobileHandMeasureRafRef.current != null) {
      cancelAnimationFrame(mobileHandMeasureRafRef.current);
      mobileHandMeasureRafRef.current = null;
    }
  }, []);

  if (mobileBattleScene) {
    const battlefieldEntry = boardZoneEntries.find((entry) => entry.zone === "battlefield");
    const battlefieldCards = battlefieldEntry?.cards || [];
    const graveyardCount = getZoneCount(player, "graveyard");
    const libraryCount = getZoneCount(player, "library");
    const handCount = getZoneCount(player, "hand");
    const exileCount = getZoneCount(player, "exile");

    return (
      <section
        ref={mobileZoneRef}
        className="mobile-battle-my-zone board-zone-bg battlefield-panel battlefield-panel--self relative z-[28] min-h-0 h-full overflow-visible"
        style={{
          "--player-accent": playerAccent?.hex || "#d8bf6a",
          "--panel-accent": playerAccent?.hex || "#b98946",
          "--player-accent-rgb": playerAccent?.rgb || "216, 191, 106",
        }}
        data-my-zone
      >
        {overlayZoneEntries.length > 0 ? (
          <div className="mobile-battle-inline-overlays">
            {overlayZoneEntries.map((entry) => {
              const activity = zoneActivity?.[entry.zone] || null;
              const displayCount = Number.isFinite(activity?.displayCount) ? activity.displayCount : entry.count;
              return (
                <div
                  key={entry.zone}
                  className={cn(
                    "mobile-battle-overlay-pill",
                    activity && formatZoneActivityClass(activity.direction)
                  )}
                >
                  <span>{entry.label}</span>
                  <span>{displayCount}</span>
                </div>
              );
            })}
          </div>
        ) : null}

        <div
          className="mobile-battle-my-zone-stage"
          data-turn-active={isActivePlayer ? "true" : "false"}
          data-mobile-hand-drop-target="board"
        >
          <div className="mobile-battle-my-zone-battlefield">
            <BattlefieldRow
              cards={battlefieldCards}
              battlefieldSide="bottom"
              paperLayoutMode="mobile-battle-bottom"
              paperMinSlotsPerRow={7}
              selectedObjectId={selectedObjectId}
              onCardClick={handleCardClick}
              onCardPointerDown={handleCardPointerDown}
              onMobileCardActionMenu={onMobileCardActionMenu}
              onMobileCardLongPress={onMobileCardLongPress}
              activatableMap={activatableMap}
              legalTargetObjectIds={legalTargetObjectIds}
              bottomSafeInset={84}
              bottomOcclusionViewportTop={mobileHandOcclusionViewportTop}
            />
          </div>
        </div>

        <div className="mobile-battle-self-identity-shell">
          <button
            type="button"
            className={cn(
              "mobile-battle-self-identity",
              isPlayerLegalTarget && canPickTargetFromBoard && "is-targetable"
            )}
            onPointerDown={handlePlayerTargetPointerDown}
            onClick={handlePlayerTargetClick}
          >
            <span className="mobile-battle-self-copy">
              <span className="mobile-battle-self-meta">
                H {handCount} G {graveyardCount} X {exileCount} D {libraryCount}
              </span>
            </span>
            <span className="mobile-battle-inline-life" aria-hidden="true">
              {player.life}
            </span>
          </button>
        </div>

        <div className="mobile-battle-self-hud">
          <div ref={mobileHandRef} className="mobile-battle-self-hand">
            <HandZone
              player={player}
              selectedObjectId={selectedObjectId}
              onInspect={onInspect}
              isExpanded
              layout="mobile-fan"
            />
          </div>
        </div>
      </section>
    );
  }

  return (
    <section
      className="board-zone-bg battlefield-panel battlefield-panel--self relative z-[28] min-h-0 h-full overflow-visible grid p-0"
      style={{
        gridTemplateRows: mergedMobileHeader
          ? "auto minmax(0,1fr)"
          : `auto ${MY_ZONE_HEADER_HEIGHT}px minmax(0,1fr)`,
        alignContent: "stretch",
        "--player-accent": playerAccent?.hex || "#d8bf6a",
        "--panel-accent": playerAccent?.hex || "#b98946",
        "--player-accent-rgb": playerAccent?.rgb || "216, 191, 106",
      }}
      data-my-zone
    >
      {!mergedMobileHeader ? (
        <div className="min-h-0 overflow-visible px-1.5 pt-1">
          <StackTimelineRail
            selectedObjectId={selectedObjectId}
            onInspectObject={onInspect}
          />
        </div>
      ) : null}
      <div className="relative min-h-0 overflow-visible">
        <div
          className={cn(
            "battlefield-panel-header relative z-[1] overflow-visible pr-2",
            mergedMobileHeader
              ? "battlefield-panel-header--merged flex min-h-[44px] items-stretch gap-1.5"
              : "flex h-full items-center gap-2"
          )}
          data-turn-priority={isPriorityPlayer ? "true" : "false"}
        >
          <div
            className={cn(
              mergedMobileHeader
                ? "my-zone-header-meta flex min-w-0 shrink-0 items-center gap-2"
                : "flex min-w-0 items-center gap-2"
            )}
            data-my-zone-header-content
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
            <div className="ml-auto flex min-w-0 items-center gap-2">
              {mergedMobileHeader ? (
                <div className="my-zone-merged-zone-meta flex items-center gap-1 text-[10px] uppercase tracking-[0.08em] text-[#bcae93] whitespace-nowrap">
                  <span className="font-bold text-[#d8cbb0]">Hand</span>
                  <span className="text-[#efe0bb]">{player.hand_size ?? 0}</span>
                  <span className="font-bold text-[#d8cbb0]">GY</span>
                  <span className="text-[#efe0bb]">{player.graveyard_size ?? 0}</span>
                </div>
              ) : (
                <ZoneCountInline player={player} />
              )}
              {!mergedMobileHeader && <ManaPool pool={player.mana_pool} />}
              {headerControls}
            </div>
          </div>
          {mergedMobileHeader ? (
            <div className="my-zone-merged-action-shell relative z-[1] min-w-0 flex-1 self-stretch">
              {embeddedActionBar}
            </div>
          ) : null}
        </div>
        {mergedMobileHeader ? (
          <StackTimelineRail
            selectedObjectId={selectedObjectId}
            onInspectObject={onInspect}
          />
        ) : null}
      </div>
      <div
        className={cn(
          "battlefield-zones-shell relative min-h-0 h-full overflow-visible",
          mobileHandRailVisible && "my-zone-mobile-body-grid",
          mobileInspectorVisible && "has-inline-inspector"
        )}
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
                      onMobileCardActionMenu={onMobileCardActionMenu}
                      onMobileCardLongPress={onMobileCardLongPress}
                      activatableMap={activatableMap}
                      legalTargetObjectIds={legalTargetObjectIds}
                      forceSingleColumn
                    />
                  </div>
                </div>
              );
            })}
          </div>
        ) : null}
        <div
          className={cn(
            mobileHandRailVisible
              ? "my-zone-mobile-board-shell min-h-0 h-full"
              : "min-h-0 h-full"
          )}
          data-mobile-hand-drop-target={mobileHandRailVisible ? "board" : undefined}
        >
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
                    alignStart={mergedMobileHeader && entry.zone === "battlefield"}
                    bottomSafeInset={mergedMobileHeader && entry.zone === "battlefield" ? 0 : undefined}
                    selectedObjectId={selectedObjectId}
                    onCardClick={handleCardClick}
                    onCardPointerDown={handleCardPointerDown}
                    onMobileCardActionMenu={mobileBattleScene && entry.zone === "battlefield" ? onMobileCardActionMenu : null}
                    onMobileCardLongPress={mobileBattleScene && entry.zone === "battlefield" ? onMobileCardLongPress : null}
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
        {mobileInspectorVisible ? (
          <aside
            className="my-zone-mobile-inspector-rail"
            data-mobile-hand-drop-target="inspector"
          >
            <div className="my-zone-mobile-inline-inspector">
              <div className="my-zone-mobile-inline-inspector-stage">
                <HoverArtOverlay
                  objectId={selectedObjectId}
                  displayMode="inspector"
                  availableInspectorWidth={182}
                  availableInspectorHeight={116}
                  hideOwnershipMetadata
                  minInspectorTextScale={0.46}
                  minInspectorTitleScale={0.42}
                  onInspectorAccentChange={null}
                />
              </div>
            </div>
          </aside>
        ) : null}
        {mobileHandRailVisible ? (
          <aside className="my-zone-hand-rail">
            <div className="my-zone-hand-rail-body">
              <HandZone
                player={player}
                selectedObjectId={selectedObjectId}
                onInspect={onInspect}
                isExpanded
                layout="vertical-rail"
              />
            </div>
          </aside>
        ) : null}
      </div>

    </section>
  );
}
