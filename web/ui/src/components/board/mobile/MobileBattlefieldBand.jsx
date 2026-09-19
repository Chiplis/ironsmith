import MobileBattlefieldLane from "@/components/board/MobileBattlefieldLane";
import MobileZonePiles from "./MobileZonePiles";
import MobileManaPool from "./MobileManaPool";
import { cn } from "@/lib/utils";

// Wraps two `MobileBattlefieldLane` rows for one player. Side="opponent" applies the
// combat band-click capture (set via the `onClickCapture` / pointer-event handlers
// passed in from MobileBattleScene). Side="self" registers as a drop target so a card
// dragged from MobileHandFan can land on the battlefield to dispatch a priority play.
export default function MobileBattlefieldBand({
  side,
  player,
  onOpenZone,
  manaIndented = false,
  rows,
  cardWidth,
  cardHeight,
  landHeight,
  selectedObjectId,
  onInspect,
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

  const laneProps = {
    battlefieldSide, selectedObjectId, onInspect, onCardClick, onCardPointerDown,
    onMobileCardActionMenu, onMobileCardLongPress, activatableMap, legalTargetObjectIds,
  };
  // Keep six pixels clear on each side of the center line without moving HUDs.
  const combatCardHeight = Math.max(24, cardHeight - 6);
  const creatureCardWidth = Math.floor(cardWidth * combatCardHeight / cardHeight);
  const resourceRow = <div className="arena-resource-row" key="resources">
    <MobileBattlefieldLane {...laneProps} cards={rows.backCards} cardHeight={landHeight}
      cardWidth={Math.floor(landHeight * 1.35)} className="arena-land-lane" />
    <div className="arena-avatar-space" aria-hidden="true" />
    <MobileBattlefieldLane {...laneProps} cards={rows.supportCards || []} cardHeight={landHeight}
      cardWidth={Math.floor(landHeight * 1.24)} className="arena-support-lane" />
  </div>;
  const combatRow = <div className="arena-combat-row" key="combat">
    <MobileBattlefieldLane {...laneProps} cards={rows.frontCards} cardHeight={combatCardHeight}
      cardWidth={creatureCardWidth} className="arena-creature-lane" />
    {rows.specialCards?.length > 0 && <MobileBattlefieldLane {...laneProps} cards={rows.specialCards}
      cardHeight={combatCardHeight} cardWidth={cardWidth} className="arena-special-lane" />}
  </div>;

  return (
    <section
      className={cn(
        "mobile-mtga-battlefield-band",
        isOpponent
          ? "mobile-mtga-battlefield-band--opponent"
          : "mobile-mtga-battlefield-band--self",
        className,
      )}
      data-mana-indented={manaIndented || undefined}
      data-has-mana={Object.values(player?.mana_pool || {}).some((amount) => Number(amount) >= 1) || undefined}
    >
      <MobileManaPool pool={player?.mana_pool} side={side} interactive={!isOpponent} className="mobile-mana-column" />
      <div className="mobile-battlefield-lanes" {...wrapperProps}>
        {isOpponent ? [resourceRow, combatRow] : [combatRow, resourceRow]}
      </div>
      <MobileZonePiles player={player} onOpenZone={onOpenZone} legalTargetObjectIds={legalTargetObjectIds} />
    </section>
  );
}
