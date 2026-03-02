import { useState } from "react";
import { cn } from "@/lib/utils";
import ManaPool from "./ManaPool";
import { ScrollArea } from "@/components/ui/scroll-area";

export default function PlayerCard({ player, isActive, isPerspective, label }) {
  const [expanded, setExpanded] = useState(false);
  const expandable = !isPerspective;
  const graveyardCards = Array.isArray(player.graveyard_cards) ? player.graveyard_cards : [];
  const exileCards = Array.isArray(player.exile_cards) ? player.exile_cards : [];

  const battlefieldCount = (player.battlefield || []).reduce((total, card) => {
    const count = Number(card.count);
    return total + (Number.isFinite(count) && count > 1 ? count : 1);
  }, 0);

  return (
    <section
      className={cn(
        "border p-2 grid gap-2",
        "bg-gradient-to-b from-[#151e2a] to-[#101723]",
        isActive && "border-[rgba(88,214,166,0.76)] shadow-[0_0_0_1px_rgba(88,214,166,0.32)_inset]",
        isPerspective && "border-[rgba(100,169,255,0.88)] shadow-[0_0_0_1px_rgba(100,169,255,0.35)_inset]",
        !isActive && !isPerspective && "border-game-line-2"
      )}
      data-player-id={player.id}
    >
      <div className="flex items-center gap-1.5 min-w-0">
        {expandable && (
          <button
            className={cn(
              "w-5 h-5 min-h-5 p-0 border text-[11px] leading-none cursor-pointer",
              expanded
                ? "border-[#5a83af] bg-[#17273a] text-[#cbe2ff]"
                : "border-[#36506e] bg-[#101a26] text-[#cbe2ff]"
            )}
            onClick={() => setExpanded((e) => !e)}
          >
            {expanded ? "\u25BE" : "\u25B8"}
          </button>
        )}
        <h2 className="text-[16px] font-bold m-0 truncate">{player.name}</h2>
        <ManaPool pool={player.mana_pool} />
      </div>

      <div className="flex flex-wrap gap-1 text-[11px] text-[#a8bfdd] tracking-wide">
        <span className="border border-[#223448] bg-[#0b121b] px-1 rounded-sm min-w-[30px] text-center">
          L {player.library_size}
        </span>
        <span className="border border-[#223448] bg-[#0b121b] px-1 rounded-sm min-w-[30px] text-center">
          H {player.hand_size}
        </span>
        <span className="border border-[#223448] bg-[#0b121b] px-1 rounded-sm min-w-[30px] text-center">
          G {player.graveyard_size}
        </span>
        <span className="border border-[#223448] bg-[#0b121b] px-1 rounded-sm min-w-[30px] text-center">
          X {exileCards.length}
        </span>
        <span className="border border-[#223448] bg-[#0b121b] px-1 rounded-sm min-w-[30px] text-center">
          B {battlefieldCount}
        </span>
      </div>

      {expanded && (
        <div className="grid gap-1.5">
          <ZoneBlock title="Graveyard" count={graveyardCards.length} cards={graveyardCards} />
          <ZoneBlock title="Exile" count={exileCards.length} cards={exileCards} />
        </div>
      )}
    </section>
  );
}

function ZoneBlock({ title, count, cards }) {
  return (
    <div className="border border-[#24374b] bg-[#0a111a] p-1 grid gap-1">
      <div className="flex justify-between items-center gap-1.5 text-[#a4bdd7] uppercase tracking-wider text-[10px] font-bold">
        <span>{title}</span>
        <span>{count}</span>
      </div>
      <ScrollArea className="max-h-[132px]">
        <div className="grid gap-1">
          {cards.length === 0 ? (
            <div className="border border-[#223244] bg-[#0d141f] text-[#8ea5c0] text-[11px] leading-tight p-1 cursor-default">
              Empty
            </div>
          ) : (
            cards.map((card) => (
              <div
                key={card.id}
                className="border border-[#2b4058] bg-gradient-to-b from-[#172334] to-[#0f1826] text-[#dbe9ff] text-[11px] leading-tight p-1 cursor-pointer hover:border-[#6f95bf] hover:bg-[#1a2a3e]"
                data-object-id={card.id}
                title={`Inspect ${card.name}`}
              >
                {card.name}
              </div>
            ))
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
