import { useGame } from "@/context/GameContext";
import BattlefieldRow from "./BattlefieldRow";
import ManaPool from "@/components/left-rail/ManaPool";
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

export default function MyZone({
  player,
  selectedObjectId,
  onInspect,
  zoneViews = ["battlefield"],
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
}) {
  const { state } = useGame();

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
    if (canPickTargetFromBoard) {
      const candidateObjectIds = [Number(card.id)];
      if (Array.isArray(card.member_ids)) {
        for (const memberId of card.member_ids) {
          candidateObjectIds.push(Number(memberId));
        }
      }
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

    // Always inspect
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
    <section className="board-zone-bg relative z-[2] p-2 min-h-[120px] overflow-visible grid gap-1" style={{ gridTemplateRows: "auto minmax(0,1fr)", alignContent: "stretch" }}>
      <div>
        <div className="relative -top-[5px] flex items-center gap-2">
          <span
            className={cn(
              "text-[23px] font-bold leading-none text-[#f5d08b] tabular-nums",
              isPlayerLegalTarget
                && "text-[#d7ebff] rounded px-1 py-0.5 shadow-[0_0_10px_rgba(100,169,255,0.5)] ring-1 ring-[#64a9ff]/55"
            )}
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
            onClick={handlePlayerTargetClick}
            style={{ cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined }}
          >
            {player.name}
            {zoneName && <span className="text-muted-foreground">{zoneName}</span>}
          </span>
          <ZoneCountInline player={player} />
          <ManaPool pool={player.mana_pool} />
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
                  <div className="flex items-center gap-1 text-[11px] uppercase tracking-wide text-[#9cb8d8] px-0.5">
                    <span>{entry.label}</span>
                    <span className="text-[#d6e6fb]">{entry.cards.length}</span>
                  </div>
                )}
                <BattlefieldRow
                  cards={entry.cards}
                  compact={entry.zone !== "battlefield"}
                  selectedObjectId={selectedObjectId}
                  onCardClick={handleCardClick}
                  activatableMap={activatableMap}
                  legalTargetObjectIds={legalTargetObjectIds}
                  allowVerticalScroll={entry.zone === "hand"}
                />
              </div>
            </div>
          );
        })}
      </div>

    </section>
  );
}
