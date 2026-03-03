import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import BattlefieldRow from "./BattlefieldRow";
import ManaPool from "@/components/left-rail/ManaPool";

function getZoneCards(player, zoneView) {
  switch (zoneView) {
    case "hand": return player.hand_cards || [];
    case "graveyard": return player.graveyard_cards || [];
    case "exile": return player.exile_cards || [];
    default: return player.battlefield || [];
  }
}

function ZoneCountChips({ player }) {
  const exileCards = Array.isArray(player.exile_cards) ? player.exile_cards : [];
  const battlefieldCount = (player.battlefield || []).reduce((total, card) => {
    const count = Number(card.count);
    return total + (Number.isFinite(count) && count > 1 ? count : 1);
  }, 0);

  return (
    <div className="flex flex-wrap gap-1 text-[11px] text-[#a8bfdd]">
      <span className="bg-[#0b121b] px-1.5 rounded-sm" title="Library">
        Lib <span className="font-bold text-[#d6e6fb]">{player.library_size}</span>
      </span>
      <span className="bg-[#0b121b] px-1.5 rounded-sm" title="Hand">
        Hand <span className="font-bold text-[#d6e6fb]">{player.hand_size}</span>
      </span>
      <span className="bg-[#0b121b] px-1.5 rounded-sm" title="Graveyard">
        GY <span className="font-bold text-[#d6e6fb]">{player.graveyard_size}</span>
      </span>
      <span className="bg-[#0b121b] px-1.5 rounded-sm" title="Exile">
        Exl <span className="font-bold text-[#d6e6fb]">{exileCards.length}</span>
      </span>
      <span className="bg-[#0b121b] px-1.5 rounded-sm" title="Battlefield">
        BF <span className="font-bold text-[#d6e6fb]">{battlefieldCount}</span>
      </span>
    </div>
  );
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
  const [zoneCounts, setZoneCounts] = useState(false);
  const cards = getZoneCards(player, zoneView);
  const zoneName = zoneView === "battlefield" ? "" : ` — ${zoneView.charAt(0).toUpperCase() + zoneView.slice(1)}`;

  return (
    <div className="bg-gradient-to-b from-[#101826] to-[#0a121d] rounded p-1.5 grid gap-1.5 min-h-0 h-full" style={{ gridTemplateRows: "auto minmax(0,1fr)", alignContent: "stretch" }}>
      <div>
        <div className="flex items-center gap-2">
          <span
            className="text-[23px] font-bold leading-none text-[#f5d08b] tabular-nums px-1 py-0.5 rounded"
            data-player-target={player.index ?? player.id}
          >
            {player.life}
          </span>
          <span
            className="text-[16px] text-[#a4bdd7] uppercase tracking-wider font-bold"
            data-player-target={player.index ?? player.id}
          >
            {player.name}
            {zoneName && <span className="text-muted-foreground">{zoneName}</span>}
          </span>
          <ManaPool pool={player.mana_pool} />
          <button
            className="p-0.5 text-muted-foreground hover:text-[#a4bdd7] transition-colors"
            onClick={() => setZoneCounts((v) => !v)}
            title="Toggle zone counts"
          >
            {zoneCounts ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" />}
          </button>
          <span className="text-[14px] text-muted-foreground ml-auto">{cards.length} cards</span>
        </div>
        {zoneCounts && (
          <div className="mt-1">
            <ZoneCountChips player={player} />
          </div>
        )}
      </div>
      <BattlefieldRow cards={cards} compact selectedObjectId={selectedObjectId} onInspect={onInspect} />
    </div>
  );
}
