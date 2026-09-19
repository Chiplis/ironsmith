import useUiText from "@/i18n/useUiText";
import ZonePileArt from "@/components/board/ZonePileArt";
import { PILE_ZONES, zonePileCards } from "@/lib/zone-piles";

export default function MobileZonePiles({ player, onOpenZone, legalTargetObjectIds }) {
  const ui = useUiText();
  if (!player) return null;
  return <aside className="mobile-zone-piles" data-player-zone-piles aria-label={ui("{0}'s zones", { 0: player.name })}>
    {PILE_ZONES.map((zone) => {
      const cards = zonePileCards(player, zone);
      const count = Number(player[`${zone}_size`] ?? cards.length);
      const label = zone === "graveyard" ? "GY" : "Exile";
      const hasTargets = cards.some((card) => legalTargetObjectIds?.has(Number(card.id)));
      return <div className="zone-pile-slot" key={zone}>
        <span className="zone-pile-label">{ui(label)} <strong>{count}</strong></span>
        <button type="button" className="zone-pile" data-zone-pile={zone}
          data-zone-owner={String(player.id ?? player.index)}
          data-has-targets={hasTargets || undefined}
          aria-label={ui("{0}'s {1}, {2} cards. Open zone", { 0: player.name, 1: ui(label), 2: count })}
          onClick={() => onOpenZone(zone, player)}>
          <ZonePileArt card={cards[0]} />
        </button>
      </div>;
    })}
  </aside>;
}
