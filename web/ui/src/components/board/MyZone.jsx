import BattlefieldRow from "./BattlefieldRow";

function getZoneCards(player, zoneView) {
  switch (zoneView) {
    case "hand": return player.hand_cards || [];
    case "graveyard": return player.graveyard_cards || [];
    case "exile": return player.exile_cards || [];
    default: return player.battlefield || [];
  }
}

export default function MyZone({ player, selectedObjectId, onInspect, zoneView = "battlefield" }) {
  const cards = getZoneCards(player, zoneView);
  const zoneName = zoneView === "battlefield" ? "" : ` — ${zoneView.charAt(0).toUpperCase() + zoneView.slice(1)}`;

  return (
    <section className="board-zone-bg p-2 min-h-[120px] overflow-hidden grid gap-1" style={{ gridTemplateRows: "auto minmax(0,1fr)", alignContent: "stretch" }}>
      <div className="flex justify-between items-baseline gap-2">
        <span className="text-[12px] text-[#a4bdd7] uppercase tracking-wider font-bold">
          {player.name} <span className="text-[#e7edf8]">({player.life})</span>
          {zoneName && <span className="text-muted-foreground">{zoneName}</span>}
        </span>
        <span className="text-[11px] text-muted-foreground">{cards.length} cards</span>
      </div>
      <BattlefieldRow cards={cards} selectedObjectId={selectedObjectId} onInspect={onInspect} />
    </section>
  );
}
