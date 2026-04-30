import BattlefieldRow from "@/components/board/BattlefieldRow";
import { cn } from "@/lib/utils";

// Extracted from MobileBattleScene's inline `BattlefieldLane` so multiple new
// region components (MobileBattlefieldBand for both opponent and self sides)
// can share the same single-row, fixed-cell layout.
export default function MobileBattlefieldLane({
  cards = [],
  cardHeight = 48,
  cardWidth = 62,
  clippedHeight = null,
  battlefieldSide,
  selectedObjectId,
  onCardClick,
  onCardPointerDown,
  onMobileCardActionMenu,
  onMobileCardLongPress,
  activatableMap,
  legalTargetObjectIds,
  className = "",
}) {
  const viewportHeight = clippedHeight ?? cardHeight;
  return (
    <div
      className={cn("mobile-mtga-battlefield-lane", className)}
      style={{ height: `${viewportHeight}px` }}
    >
      <div
        className="mobile-mtga-battlefield-lane-track"
        style={{ height: `${cardHeight}px` }}
      >
        <BattlefieldRow
          cards={cards}
          battlefieldSide={battlefieldSide}
          paperLayoutMode="single-row"
          layoutOverride={{
            rows: 1,
            cols: Math.max(1, cards.length),
            cardWidth,
            cardHeight,
            overlapPx: 0,
          }}
          selectedObjectId={selectedObjectId}
          onCardClick={onCardClick}
          onCardPointerDown={onCardPointerDown}
          onMobileCardActionMenu={onMobileCardActionMenu}
          onMobileCardLongPress={onMobileCardLongPress}
          activatableMap={activatableMap}
          legalTargetObjectIds={legalTargetObjectIds}
        />
      </div>
    </div>
  );
}
