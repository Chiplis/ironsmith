import BattlefieldRow from "./BattlefieldRow";

function getZoneCards(player, zoneView) {
  switch (zoneView) {
    case "hand": return player.hand_cards || [];
    case "graveyard": return player.graveyard_cards || [];
    case "exile": return player.exile_cards || [];
    default: return player.battlefield || [];
  }
}

export default function OpponentZone({ opponents, selectedObjectId, onInspect, zoneView = "battlefield" }) {
  if (!opponents.length) return <section className="board-zone-bg p-1.5 min-h-0" />;

  return (
    <section className="board-zone-bg p-1.5 min-h-0 overflow-hidden" style={{ alignContent: "stretch" }}>
      <div
        className="grid gap-2 min-h-0 h-full"
        style={{
          gridTemplateColumns: `repeat(auto-fit, minmax(220px, 1fr))`,
          gridAutoRows: "minmax(0, 1fr)",
          alignContent: "stretch",
        }}
      >
        {opponents.map((player) => (
          <OpponentSlot key={player.id} player={player} selectedObjectId={selectedObjectId} onInspect={onInspect} zoneView={zoneView} />
        ))}
      </div>
    </section>
  );
}

function OpponentSlot({ player, selectedObjectId, onInspect, zoneView }) {
  const cards = getZoneCards(player, zoneView);
  const zoneName = zoneView === "battlefield" ? "" : ` — ${zoneView.charAt(0).toUpperCase() + zoneView.slice(1)}`;

  return (
    <div className="border border-[#2b3e55] bg-gradient-to-b from-[#101826] to-[#0a121d] p-1.5 grid gap-1.5 min-h-0 h-full" style={{ gridTemplateRows: "auto minmax(0,1fr)", alignContent: "stretch" }}>
      <div className="flex justify-between items-baseline gap-2">
        <span className="text-[12px] text-[#a4bdd7] uppercase tracking-wider font-bold">
          {player.name} <span className="text-[#e7edf8]">({player.life})</span>
          {zoneName && <span className="text-muted-foreground">{zoneName}</span>}
        </span>
        <span className="text-[11px] text-muted-foreground">{cards.length} cards</span>
      </div>
      <BattlefieldRow cards={cards} compact selectedObjectId={selectedObjectId} onInspect={onInspect} />
    </div>
  );
}
