import { useLayoutEffect, useRef, useState } from "react";
import BattlefieldRow from "@/components/board/BattlefieldRow";
import { MOBILE_BATTLE_CARD_ASPECT_RATIO } from "@/lib/mobile-battle-layout";
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
  onInspect,
  onCardClick,
  onCardPointerDown,
  onMobileCardActionMenu,
  onMobileCardLongPress,
  activatableMap,
  legalTargetObjectIds,
  className = "",
}) {
  const laneRef = useRef(null);
  const [laneWidth, setLaneWidth] = useState(0);
  useLayoutEffect(() => {
    const lane = laneRef.current;
    const update = () => setLaneWidth(lane.clientWidth);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(lane);
    return () => observer.disconnect();
  }, []);
  // Fit ordinary boards; keep crowded boards readable with horizontal scrolling.
  const fittedWidth = laneWidth ? Math.min(cardWidth, Math.max(40, Math.floor((laneWidth - Math.max(0, Math.min(cards.length, 7) - 1) * 2) / Math.max(1, Math.min(cards.length, 7))))) : cardWidth;
  const fittedHeight = Math.min(cardHeight, Math.floor(fittedWidth / (cardWidth / cardHeight || MOBILE_BATTLE_CARD_ASPECT_RATIO)));
  const viewportHeight = clippedHeight ?? cardHeight;
  return (
    <div
      ref={laneRef}
      className={cn("mobile-mtga-battlefield-lane", className)}
      style={{ height: `${viewportHeight}px`, "--arena-card-height": `${fittedHeight}px` }}
    >
      <div
        className="mobile-mtga-battlefield-lane-track"
        style={{ height: `${fittedHeight}px`, minWidth: `${cards.length * fittedWidth + Math.max(0, cards.length - 1) * 2}px` }}
      >
        <BattlefieldRow
          cards={cards}
          battlefieldSide={battlefieldSide}
          paperLayoutMode="single-row"
          layoutOverride={{
            rows: 1,
            cols: Math.max(1, cards.length),
            cardWidth: fittedWidth,
            cardHeight: fittedHeight,
            gap: 2,
            overlapPx: 0,
          }}
          selectedObjectId={selectedObjectId}
          onInspect={onInspect}
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
