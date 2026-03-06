import { useCallback } from "react";
import BattlefieldRow from "./BattlefieldRow";
import ManaPool from "@/components/left-rail/ManaPool";
import { useCombatArrows } from "@/context/CombatArrowContext";
import { useGame } from "@/context/GameContext";
import { cn } from "@/lib/utils";

const ZONE_ORDER = ["battlefield", "hand", "graveyard", "exile"];
const ZONE_LABELS = {
  battlefield: "Battlefield",
  hand: "Hand",
  graveyard: "Graveyard",
  exile: "Exile",
};

function normalizeZoneViews(zoneViews) {
  const normalized = Array.isArray(zoneViews)
    ? zoneViews.filter((zone) => ZONE_ORDER.includes(zone))
    : [];
  return normalized.length > 0 ? normalized : ["battlefield"];
}

function getZoneCards(player, zone) {
  switch (zone) {
    case "hand": return player.hand_cards || [];
    case "graveyard": return player.graveyard_cards || [];
    case "exile": return player.exile_cards || [];
    default: return player.battlefield || [];
  }
}

function buildZoneEntries(player, zoneViews) {
  const activeZones = normalizeZoneViews(zoneViews);
  return ZONE_ORDER.map((zone) => ({
    zone,
    label: ZONE_LABELS[zone] || zone,
    cards: getZoneCards(player, zone),
    active: activeZones.includes(zone),
  }));
}

function zoneCounts(player) {
  const exileCards = Array.isArray(player.exile_cards) ? player.exile_cards : [];
  const battlefieldCount = (player.battlefield || []).reduce((total, card) => {
    const count = Number(card.count);
    return total + (Number.isFinite(count) && count > 1 ? count : 1);
  }, 0);

  return [
    { label: "Battlefield", count: battlefieldCount },
    { label: "Hand", count: player.hand_size ?? 0 },
    { label: "Graveyard", count: player.graveyard_size ?? 0 },
    { label: "Exile", count: exileCards.length },
  ];
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

function actionSignature(action) {
  return `${action?.kind || ""}|${action?.from_zone || ""}|${String(action?.label || "").trim().toLowerCase()}`;
}

function resolveSinglePriorityCardAction(state, card) {
  const decision = state?.decision;
  if (!decision || decision.kind !== "priority" || decision.player !== state?.perspective) {
    return null;
  }

  const objectIds = collectCardObjectIds(card);
  if (objectIds.length === 0) return null;
  const objectIdSet = new Set(objectIds.map((id) => String(id)));
  const candidateActions = (decision.actions || []).filter((action) =>
    action.kind !== "pass_priority"
    && action.object_id != null
    && objectIdSet.has(String(action.object_id))
  );
  if (candidateActions.length === 0) return null;

  const uniqueActionKinds = new Set(candidateActions.map(actionSignature));
  if (uniqueActionKinds.size !== 1) return null;
  return candidateActions[0];
}

function ZoneCountInline({ player }) {
  const counts = zoneCounts(player);
  return (
    <div className="flex items-center gap-2 text-[11px] uppercase tracking-wide text-[#8ea8c8] whitespace-nowrap">
      {counts.map((entry) => (
        <span key={entry.label}>
          <span className="font-bold text-[#c1d4ea]">{entry.label}</span>{" "}
          <span className="text-[#d6e6fb] font-semibold">{entry.count}</span>
        </span>
      ))}
    </div>
  );
}

export default function OpponentZone({
  opponents,
  selectedObjectId,
  onInspect,
  onExpandInspector,
  zoneViews = ["battlefield"],
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
}) {
  const { state, dispatch } = useGame();
  if (!opponents.length) return <section className="board-zone-bg p-1.5 min-h-0" />;

  return (
    <section className="board-zone-bg relative z-[2] p-1.5 min-h-0 overflow-visible" data-opponents-zones style={{ alignContent: "stretch" }}>
      <div
        className="grid gap-2 min-h-0 h-full"
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
            onExpandInspector={onExpandInspector}
            zoneViews={zoneViews}
            state={state}
            dispatch={dispatch}
            legalTargetPlayerIds={legalTargetPlayerIds}
            legalTargetObjectIds={legalTargetObjectIds}
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
  onExpandInspector,
  zoneViews,
  state,
  dispatch,
  legalTargetPlayerIds,
  legalTargetObjectIds,
}) {
  const { combatModeRef, combatMode, dragArrow } = useCombatArrows();
  const zoneEntries = buildZoneEntries(player, zoneViews);
  const activeZoneEntries = zoneEntries.filter((entry) => entry.active);
  const visibleZones = new Set(
    activeZoneEntries
      .filter((entry) => entry.zone === "battlefield" || entry.cards.length > 0)
      .map((entry) => entry.zone)
  );
  if (visibleZones.size === 0 && activeZoneEntries.length > 0) {
    visibleZones.add(activeZoneEntries[0].zone);
  }
  const zoneName = activeZoneEntries.length === 1
    ? (activeZoneEntries[0].zone === "battlefield" ? "" : ` — ${activeZoneEntries[0].label}`)
    : "";
  const showZoneHeaders = visibleZones.size > 1;
  const playerIdx = player.index ?? player.id;
  const isPlayerLegalTarget =
    legalTargetPlayerIds.has(Number(player.id)) || legalTargetPlayerIds.has(Number(player.index));
  const canPickTargetFromBoard = state?.decision?.kind === "targets"
    && state?.decision?.player === state?.perspective;
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

    const singlePriorityAction = resolveSinglePriorityCardAction(state, card);
    if (singlePriorityAction) {
      dispatch(
        { type: "priority_action", action_index: singlePriorityAction.index },
        singlePriorityAction.label
      );
      return;
    }

    onInspect?.(card.id);
  };

  const handlePlayerTargetClick = () => {
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
  };

  return (
    <div
      className={cn(
        "bg-gradient-to-b from-[#101826] to-[#0a121d] rounded p-1.5 grid gap-1.5 min-h-0 h-full",
        zoneIsAttackHoverTarget && "attack-target-zone"
      )}
      style={{ gridTemplateRows: "auto minmax(0,1fr)", alignContent: "stretch", cursor: attackerArrowActive ? "crosshair" : undefined }}
      data-opponent-zone={playerIdx}
      onClickCapture={handleClickCapture}
    >
      <div>
        <div className="relative -top-[5px] flex items-center gap-2">
          <span
            className={cn(
              "text-[23px] font-bold leading-none text-[#f5d08b] tabular-nums px-1 py-0.5 rounded",
              isPlayerLegalTarget
                && "text-[#d7ebff] shadow-[0_0_10px_rgba(100,169,255,0.5)] ring-1 ring-[#64a9ff]/55"
            )}
            data-player-target={player.index ?? player.id}
            onClick={handlePlayerTargetClick}
            style={{ cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined }}
          >
            {player.life}
          </span>
          <span
            className={cn(
              "text-[16px] text-[#a4bdd7] uppercase tracking-wider font-bold",
              isPlayerLegalTarget && "text-[#d7ebff] drop-shadow-[0_0_7px_rgba(100,169,255,0.7)]"
            )}
            data-player-target={player.index ?? player.id}
            data-player-target-name={player.index ?? player.id}
            onClick={handlePlayerTargetClick}
            style={{ cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined }}
          >
            {player.name}
            {zoneName && <span className="text-muted-foreground">{zoneName}</span>}
          </span>
          <ManaPool pool={player.mana_pool} />
          <div className="ml-auto">
            <ZoneCountInline player={player} />
          </div>
        </div>
      </div>
      <div className="flex gap-1 min-h-0 h-full overflow-visible">
        {zoneEntries.map((entry) => {
          const isVisible = entry.active && visibleZones.has(entry.zone);
          return (
            <div
              key={entry.zone}
              className="min-h-0 h-full"
              style={{
                flexGrow: isVisible ? 1 : 0,
                flexShrink: 1,
                flexBasis: "0%",
                maxWidth: isVisible ? "100%" : "0px",
                opacity: isVisible ? 1 : 0,
                transform: isVisible ? "translateY(0)" : "translateY(4px)",
                pointerEvents: isVisible ? "auto" : "none",
                overflow: isVisible ? "visible" : "hidden",
                transition: "flex-grow 220ms ease, max-width 220ms ease, opacity 180ms ease, transform 220ms ease",
              }}
            >
              <div
                className="grid gap-1 min-h-0 h-full"
                style={{ gridTemplateRows: showZoneHeaders ? "auto minmax(0,1fr)" : "minmax(0,1fr)" }}
              >
                {showZoneHeaders && (
                  <div className="flex items-center gap-1 text-[10px] uppercase tracking-wide text-[#9cb8d8] px-0.5">
                    <span>{entry.label}</span>
                    <span className="text-[#d6e6fb]">{entry.cards.length}</span>
                  </div>
                )}
                <BattlefieldRow
                  cards={entry.cards}
                  compact={entry.zone !== "battlefield"}
                  battlefieldSide="top"
                  selectedObjectId={selectedObjectId}
                  onCardClick={handleCardClick}
                  onExpandInspector={entry.zone === "battlefield" ? onExpandInspector : undefined}
                  legalTargetObjectIds={legalTargetObjectIds}
                  allowVerticalScroll={entry.zone === "hand"}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
