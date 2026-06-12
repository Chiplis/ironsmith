import MobileBattlefieldLane from "@/components/board/MobileBattlefieldLane";
import { cn } from "@/lib/utils";

// Wraps two `MobileBattlefieldLane` rows for one player. Side="opponent" applies the
// combat band-click capture (set via the `onClickCapture` / pointer-event handlers
// passed in from MobileBattleScene). Side="self" registers as a drop target so a card
// dragged from MobileHandFan can land on the battlefield to dispatch a priority play.
export default function MobileBattlefieldBand({
  side,
  rows,
  cardWidth,
  cardHeight,
  selfBackVisibleHeight,
  selectedObjectId,
  onCardClick,
  onCardPointerDown,
  onMobileCardActionMenu,
  onMobileCardLongPress,
  activatableMap,
  legalTargetObjectIds,
  // Opponent-band capture handlers — wired up in MobileBattleScene to handle combat
  // target clicks and tap-to-attack hit-testing.
  onPointerDownCapture,
  onPointerUpCapture,
  onPointerCancelCapture,
  onPointerLeave,
  onClickCapture,
  className,
}) {
  const isOpponent = side === "opponent";
  const battlefieldSide = isOpponent ? "top" : "bottom";
  const wrapperProps = isOpponent
    ? {
        onPointerDownCapture,
        onPointerUpCapture,
        onPointerCancelCapture,
        onPointerLeave,
        onClickCapture,
        "data-mobile-hand-drop-target": "battlefield",
      }
    : { "data-mobile-hand-drop-target": "battlefield" };

  // Opponent: back row above front row. Self: front row above back row (back row clipped).
  const lanes = isOpponent
    ? [
        {
          key: "back",
          cards: rows.backCards,
          clippedHeight: null,
          laneClass: "mobile-mtga-battlefield-lane--opponent-back",
        },
        {
          key: "front",
          cards: rows.frontCards,
          clippedHeight: null,
          laneClass: "mobile-mtga-battlefield-lane--opponent-front",
        },
      ]
    : [
        {
          key: "front",
          cards: rows.frontCards,
          clippedHeight: null,
          laneClass: "mobile-mtga-battlefield-lane--self-front",
        },
        {
          key: "back",
          cards: rows.backCards,
          clippedHeight: selfBackVisibleHeight,
          laneClass: "mobile-mtga-battlefield-lane--self-back",
        },
      ];

  return (
    <section
      className={cn(
        "mobile-mtga-battlefield-band",
        isOpponent
          ? "mobile-mtga-battlefield-band--opponent"
          : "mobile-mtga-battlefield-band--self",
        className,
      )}
      {...wrapperProps}
    >
      {lanes.map(({ key, cards, clippedHeight, laneClass }) => (
        <MobileBattlefieldLane
          key={key}
          cards={cards}
          cardHeight={cardHeight}
          cardWidth={cardWidth}
          clippedHeight={clippedHeight}
          battlefieldSide={battlefieldSide}
          selectedObjectId={selectedObjectId}
          onCardClick={onCardClick}
          onCardPointerDown={onCardPointerDown}
          onMobileCardActionMenu={onMobileCardActionMenu}
          onMobileCardLongPress={onMobileCardLongPress}
          activatableMap={activatableMap}
          legalTargetObjectIds={legalTargetObjectIds}
          className={laneClass}
        />
      ))}
    </section>
  );
}
