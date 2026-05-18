import { useCallback, useEffect, useState } from "react";
import BattlefieldRow from "./BattlefieldRow";
import ManaPool from "@/components/left-rail/ManaPool";
import { useCombatArrows } from "@/context/useCombatArrows";
import { useGame } from "@/context/GameContext";
import { getPlayerAccent } from "@/lib/player-colors";
import { cn } from "@/lib/utils";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";

const ZONE_ORDER = ["battlefield", "hand", "graveyard", "library", "exile", "command"];
const ZONE_LABELS = {
  battlefield: "Battlefield",
  hand: "Hand",
  graveyard: "GY",
  library: "Deck",
  exile: "Exile",
  command: "CZ",
};

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

function shouldShowZoneBody(player, entry, activity = null) {
  if (!entry?.active) return false;
  if (entry.zone === "library") return false;
  if (activity) return true;
  if (entry.zone === "battlefield") return true;
  if (entry.zone === "hand") {
    return Boolean(player?.can_view_hand) || (entry.cards || []).length > 0 || entry.count > 0;
  }
  if (entry.zone === "graveyard" || entry.zone === "exile") return true;
  return entry.count > 0 || (entry.cards || []).length > 0;
}

function zoneCounts(player) {
  const exileCards = Array.isArray(player.exile_cards) ? player.exile_cards : [];
  const commandCards = Array.isArray(player.command_cards) ? player.command_cards : [];
  const battlefieldCount = (player.battlefield || []).reduce((total, card) => {
    const count = Number(card.count);
    return total + (Number.isFinite(count) && count > 1 ? count : 1);
  }, 0);

  return [
    { label: "BF", title: "Battlefield", count: battlefieldCount },
    { label: "Hand", title: "Hand", count: player.hand_size ?? 0 },
    { label: "GY", title: "Graveyard", count: player.graveyard_size ?? 0 },
    { label: "Deck", title: "Library", count: player.library_size ?? 0 },
    { label: "Exl", title: "Exile", count: exileCards.length },
    { label: "CZ", title: "Command Zone", count: player.command_size ?? commandCards.length },
  ];
}

function isBaseVisibleZone(zone, zoneViews, count) {
  const baseViews = normalizeZoneViews(zoneViews);
  if (!baseViews.includes(zone)) return false;
  return zone === "battlefield" || count > 0;
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
      (action.kind === "activate_ability"
        || action.kind === "activate_mana_ability"
        || action.kind === "untap_land")
      && action.object_id != null
    ) {
      const objId = Number(action.object_id);
      if (!activatableMap.has(objId)) activatableMap.set(objId, []);
      activatableMap.get(objId).push(action);
    }
  }

  return activatableMap;
}

function ZoneCountInline({ player, onOpenDecklist = null }) {
  const counts = zoneCounts(player);
  const libraryTopName = player?.can_view_library_top ? String(player?.library_top || "Empty") : "";
  return (
    <div className="battlefield-counts flex items-center gap-2 text-[11px] uppercase tracking-wide text-[#8ea8c8] whitespace-nowrap">
      {counts.map((entry) => {
        const showLibraryTop = entry.label === "Deck" && libraryTopName;
        const deckEntry = entry.label === "Deck" && typeof onOpenDecklist === "function";
        const content = (
          <>
            <span className="battlefield-count-label font-bold text-[#c1d4ea]">{entry.label}</span>
            <span className="text-[#d6e6fb] font-semibold">{entry.count}</span>
            {showLibraryTop && (
              <span className="battlefield-count-top text-[#f0dfba] font-semibold">({libraryTopName})</span>
            )}
          </>
        );
        if (deckEntry) {
          return (
            <button
              key={entry.label}
              type="button"
              className={cn(
                "battlefield-count-item cursor-pointer text-left transition-colors hover:border-[#6d8ead] hover:text-[#e5f2ff]",
                showLibraryTop && "battlefield-count-item--with-top"
              )}
              title="Open decklist"
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onOpenDecklist(player);
              }}
            >
              {content}
            </button>
          );
        }
        return (
          <span
            key={entry.label}
            className={cn("battlefield-count-item", showLibraryTop && "battlefield-count-item--with-top")}
            title={showLibraryTop ? `Top card: ${libraryTopName}` : entry.title}
          >
            {content}
          </span>
        );
      })}
    </div>
  );
}

function HiddenHandRows({ count }) {
  const hiddenCount = Math.max(0, Math.floor(Number(count) || 0));
  return (
    <div className="zone-hidden-card-list" aria-label={`${hiddenCount} hidden cards`}>
      {Array.from({ length: hiddenCount }).map((_, index) => (
        <div key={index} className="zone-hidden-card-row">
          <span>Hidden</span>
        </div>
      ))}
    </div>
  );
}

function ZoneCardNameRows({
  cards = [],
  selectedObjectId = null,
  onCardClick,
  onCardPointerDown,
}) {
  if (!Array.isArray(cards) || cards.length === 0) {
    return (
      <div className="zone-card-name-list zone-card-name-list--empty">
        <div className="zone-card-name-empty">Empty</div>
      </div>
    );
  }

  return (
    <div className="zone-card-name-list">
      {cards.map((card) => {
        const objectIds = collectCardObjectIds(card).map((id) => String(id));
        const selected = selectedObjectId != null && objectIds.includes(String(selectedObjectId));
        return (
          <button
            key={card.id}
            type="button"
            className={cn("zone-card-name-row", selected && "is-selected")}
            onPointerDown={(event) => onCardPointerDown?.(event, card)}
            onClick={(event) => onCardClick?.(event, card)}
            title={String(card?.name || "Card")}
          >
            <span>{card?.name || "Card"}</span>
          </button>
        );
      })}
    </div>
  );
}

export default function OpponentZone({
  opponents,
  selectedObjectId,
  onInspect,
  onOpenDecklist = null,
  zoneViews = ["battlefield"],
  zoneActivityByPlayer = {},
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
  mobileViewport = false,
  mobileBattleScene = false,
  activeOpponentIndex: controlledActiveOpponentIndex,
  setActiveOpponentIndex: controlledSetActiveOpponentIndex,
  onMobileCardActionMenu = null,
  onMobileCardLongPress = null,
}) {
  const { state } = useGame();
  const [activeOpponentIndex, setActiveOpponentIndex] = useState(0);
  const resolvedActiveOpponentIndex = typeof controlledActiveOpponentIndex === "number"
    ? controlledActiveOpponentIndex
    : activeOpponentIndex;
  const setResolvedActiveOpponentIndex = controlledSetActiveOpponentIndex || setActiveOpponentIndex;

  useEffect(() => {
    setResolvedActiveOpponentIndex((currentIndex) => {
      if (opponents.length <= 1) return 0;
      return Math.min(currentIndex, opponents.length - 1);
    });
  }, [opponents.length, setResolvedActiveOpponentIndex]);

  if (!opponents.length) return <section className="board-zone-bg battlefield-panel battlefield-panel--opponents p-0 min-h-0" />;

  if (mobileViewport) {
    const activeOpponent = opponents[Math.min(resolvedActiveOpponentIndex, opponents.length - 1)] || opponents[0];

    return (
      <section
        className="board-zone-bg battlefield-panel battlefield-panel--opponents relative z-[2] p-0 min-h-0 overflow-visible"
        data-opponents-zones
        style={{ alignContent: "stretch" }}
      >
        <div className="h-full min-h-0">
          <OpponentSlot
            player={activeOpponent}
            selectedObjectId={selectedObjectId}
            onInspect={onInspect}
            onOpenDecklist={onOpenDecklist}
            zoneViews={zoneViews}
            zoneActivity={zoneActivityByPlayer[String(activeOpponent?.id ?? activeOpponent?.index ?? "")] || {}}
            state={state}
            legalTargetPlayerIds={legalTargetPlayerIds}
            legalTargetObjectIds={legalTargetObjectIds}
            mobileViewport
            mobileBattleScene={mobileBattleScene}
            onMobileCardActionMenu={onMobileCardActionMenu}
            onMobileCardLongPress={onMobileCardLongPress}
          />
        </div>
      </section>
    );
  }

  return (
    <section className="board-zone-bg battlefield-panel battlefield-panel--opponents relative z-[2] p-0 min-h-0 overflow-visible" data-opponents-zones style={{ alignContent: "stretch" }}>
      <div
        className="battlefield-opponents-grid grid gap-2 min-h-0 h-full"
        style={{
          gridTemplateColumns: `repeat(auto-fit, minmax(220px, 1fr))`,
          gridAutoRows: "minmax(0, 1fr)",
          alignContent: "stretch",
        }}
      >
        {opponents.map((player) => (
          <OpponentSlot
            key={player.id}
            player={player}
            selectedObjectId={selectedObjectId}
            onInspect={onInspect}
            onOpenDecklist={onOpenDecklist}
            zoneViews={zoneViews}
            zoneActivity={zoneActivityByPlayer[String(player?.id ?? player?.index ?? "")] || {}}
            state={state}
            legalTargetPlayerIds={legalTargetPlayerIds}
            legalTargetObjectIds={legalTargetObjectIds}
            onMobileCardActionMenu={onMobileCardActionMenu}
            onMobileCardLongPress={onMobileCardLongPress}
          />
        ))}
      </div>
    </section>
  );
}

function OpponentSlot({
  player,
  selectedObjectId,
  onInspect,
  onOpenDecklist = null,
  zoneViews,
  zoneActivity = {},
  state,
  legalTargetPlayerIds,
  legalTargetObjectIds,
  headerControls = null,
  mobileViewport = false,
  mobileBattleScene = false,
  onMobileCardActionMenu = null,
  onMobileCardLongPress = null,
}) {
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const { combatModeRef, combatMode, dragArrow } = useCombatArrows();
  const { playerAccentOverrides } = useGame();
  const playerAccent = getPlayerAccent(
    state?.players || [],
    player?.id,
    state?.perspective,
    playerAccentOverrides
  );
  const transientZoneViews = Object.keys(zoneActivity || {});
  const zoneEntries = buildZoneEntries(player, [...zoneViews, ...transientZoneViews]);
  const activeZoneEntries = zoneEntries.filter((entry) => entry.active);
  const displayZoneEntries = activeZoneEntries.filter((entry) =>
    shouldShowZoneBody(player, entry, zoneActivity?.[entry.zone] || null)
  );
  const supportZoneEntries = displayZoneEntries.filter((entry) => entry.zone !== "battlefield");
  const denseSupportLayout = supportZoneEntries.length > 0;
  const boardZoneEntries = displayZoneEntries.filter((entry) => entry.zone === "battlefield");
  const overlayZoneEntries = [];
  const battlefieldZoneEntry = boardZoneEntries.find((entry) => entry.zone === "battlefield");
  const shelfZoneEntries = denseSupportLayout
    ? supportZoneEntries
    : [];
  const visibleZones = new Set(
    boardZoneEntries
      .filter((entry) =>
        entry.zone === "battlefield"
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
  const playerIdx = player.index ?? player.id;
  const isActivePlayer = Number(state?.active_player) === Number(player?.id);
  const isPriorityPlayer = Number(state?.priority_player) === Number(player?.id);
  const isPlayerLegalTarget =
    legalTargetPlayerIds.has(Number(player.id)) || legalTargetPlayerIds.has(Number(player.index));
  const canPickTargetFromBoard = state?.decision?.kind === "targets"
    && samePlayerId(state?.decision?.player, state?.perspective);
  const activatableMap = buildActivatableMap(state?.decision);
  const activeAttackerId = (
    combatMode?.mode === "attackers"
      ? Number(combatMode?.selectedAttacker ?? dragArrow?.fromId ?? NaN)
      : NaN
  );
  const zoneIsAttackHoverTarget = (
    combatMode?.mode === "attackers" &&
    Number.isFinite(activeAttackerId) &&
    !!combatMode?.validTargetPlayersByAttacker?.[activeAttackerId]?.has?.(Number(playerIdx))
  );
  const attackerArrowActive = (
    combatMode?.mode === "attackers" &&
    (combatMode?.selectedAttacker != null || dragArrow?.fromId != null)
  );

  // Capture-phase click handler: when a selected attacker is awaiting a target,
  // clicking anywhere on this opponent's zone assigns the target.
  // Planeswalker is targeted only if the click is exactly on a planeswalker card.
  const handleClickCapture = useCallback((e) => {
    const cm = combatModeRef.current;
    if (!cm?.onTargetAreaClick || cm.selectedAttacker == null) return;

    e.stopPropagation();
    e.preventDefault();

    // Check if click was exactly on a card (could be a planeswalker)
    const el = document.elementFromPoint(e.clientX, e.clientY);
    const cardEl = el?.closest(".game-card[data-object-id]");
    const planeswalkerObjId = cardEl ? Number(cardEl.dataset.objectId) : null;

    cm.onTargetAreaClick(playerIdx, planeswalkerObjId);
  }, [combatModeRef, playerIdx]);

  const handleCardClick = (e, card) => {
    if (canPickTargetFromBoard && !shouldHandleClick(e)) return;
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
    <div
      className={cn(
        "battlefield-subpanel rounded-none grid min-h-0 h-full overflow-hidden",
        zoneIsAttackHoverTarget && "attack-target-zone",
        mobileBattleScene && "mobile-battle-opponent-slot"
      )}
      style={{
        gridTemplateRows: mobileViewport ? "auto minmax(0,1fr)" : "auto minmax(0,1fr) auto",
        alignContent: "stretch",
        cursor: attackerArrowActive ? "crosshair" : undefined,
        "--player-accent": playerAccent?.hex || "#d8bf6a",
        "--panel-accent": playerAccent?.hex || "#d8bf6a",
        "--player-accent-rgb": playerAccent?.rgb || "216, 191, 106",
      }}
      data-opponent-zone={playerIdx}
      onClickCapture={handleClickCapture}
    >
      <div>
        {!mobileViewport ? (
          <div
            className="battlefield-panel-header battlefield-panel-header--compact flex min-w-0 items-center gap-2 overflow-hidden"
            data-turn-priority={isPriorityPlayer ? "true" : "false"}
          >
            <span
              className={cn(
                "battlefield-life text-[23px] font-bold leading-none text-[#f5d08b] tabular-nums px-1 py-0.5 rounded-none",
                isPlayerLegalTarget
                  && "text-[#d7ebff] shadow-[0_0_10px_rgba(100,169,255,0.5)] ring-1 ring-[#64a9ff]/55"
              )}
              data-player-target={player.index ?? player.id}
              onPointerDown={handlePlayerTargetPointerDown}
              onClick={handlePlayerTargetClick}
              style={{ cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined }}
            >
              {player.life}
            </span>
            <span
              className={cn(
                "battlefield-name min-w-0 text-[16px] uppercase tracking-wider font-bold",
                isPlayerLegalTarget && "drop-shadow-[0_0_7px_rgba(100,169,255,0.7)]"
              )}
              data-player-target={player.index ?? player.id}
              data-player-target-name={player.index ?? player.id}
              onPointerDown={handlePlayerTargetPointerDown}
              onClick={handlePlayerTargetClick}
              style={{
                color: playerAccent?.hex,
                cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined,
              }}
            >
              <span className={cn(isActivePlayer && "battlefield-name-text--active")}>
                {playerDisplayName(state?.players || [], player)}
              </span>
              {zoneName && <span className="text-muted-foreground">{zoneName}</span>}
            </span>
            <div className="ml-auto flex min-w-0 flex-1 items-center justify-end gap-2">
              <ZoneCountInline player={player} onOpenDecklist={onOpenDecklist} />
              {headerControls}
            </div>
          </div>
        ) : null}
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
                      battlefieldSide="top"
                      selectedObjectId={selectedObjectId}
                      onCardClick={handleCardClick}
                      onCardPointerDown={handleCardPointerDown}
                      onMobileCardActionMenu={mobileBattleScene ? onMobileCardActionMenu : null}
                      onMobileCardLongPress={mobileBattleScene ? onMobileCardLongPress : null}
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
            "battlefield-zone-strip min-h-0 h-full overflow-visible",
            denseSupportLayout ? "battlefield-zone-strip--shelf" : "flex gap-1"
          )}
          data-zone-layout={denseSupportLayout ? "shelf" : "lanes"}
        >
        {(denseSupportLayout && battlefieldZoneEntry
          ? [battlefieldZoneEntry]
          : boardZoneEntries
        ).map((entry) => {
          const isVisible = entry.active && visibleZones.has(entry.zone);
          const isPrimaryBattlefield = entry.zone === "battlefield";
          const isCompactSideZone = entry.zone === "command";
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
                  <div className="battlefield-zone-label flex items-center gap-1 text-[10px] uppercase tracking-wide text-[#9cb8d8] px-0.5">
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
                <BattlefieldRow
                  cards={displayCards}
                  compact={entry.zone !== "battlefield"}
                  battlefieldSide="top"
                  paperLayoutMode={mobileBattleScene && entry.zone === "battlefield" ? "mobile-battle-top" : "default"}
                  paperMinSlotsPerRow={mobileBattleScene && entry.zone === "battlefield" ? 7 : null}
                  selectedObjectId={selectedObjectId}
                  onCardClick={handleCardClick}
                  onCardPointerDown={handleCardPointerDown}
                  onMobileCardActionMenu={mobileBattleScene && entry.zone === "battlefield" ? onMobileCardActionMenu : null}
                  onMobileCardLongPress={mobileBattleScene && entry.zone === "battlefield" ? onMobileCardLongPress : null}
                  activatableMap={activatableMap}
                  legalTargetObjectIds={legalTargetObjectIds}
                  allowVerticalScroll={entry.zone === "hand"}
                />
              </div>
            </div>
          );
        })}
        {denseSupportLayout && shelfZoneEntries.length > 0 ? (
          <div
            className="battlefield-zone-shelf min-h-0 h-full"
            style={{ "--zone-shelf-count": shelfZoneEntries.length }}
          >
            {shelfZoneEntries.map((entry) => {
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
                    "battlefield-zone-entry battlefield-zone-entry--shelf min-h-0",
                    activity && formatZoneActivityClass(activity.direction)
                  )}
                >
                  <div
                    className={cn(
                      "grid gap-1 min-h-0 h-full",
                      isTransientReveal && "zone-reveal-content zone-reveal-content-enter"
                    )}
                    style={{ gridTemplateRows: "auto minmax(0,1fr)" }}
                  >
                    <div className="battlefield-zone-label flex items-center gap-1 text-[10px] uppercase tracking-wide text-[#9cb8d8] px-0.5">
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
                    {entry.zone === "hand" && displayCards.length === 0 && displayCount > 0 ? (
                      <HiddenHandRows count={displayCount} />
                    ) : (
                      <ZoneCardNameRows
                        cards={displayCards}
                        selectedObjectId={selectedObjectId}
                        onCardClick={handleCardClick}
                        onCardPointerDown={handleCardPointerDown}
                      />
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        ) : null}
        </div>
      </div>
      {!mobileViewport ? (
        <div className="opponent-battlefield-mana-row">
          <ManaPool
            pool={player.mana_pool}
            alwaysVisible
            compact
            className="opponent-battlefield-mana"
          />
        </div>
      ) : null}
    </div>
  );
}
