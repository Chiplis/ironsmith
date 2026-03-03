import { cn } from "@/lib/utils";
import ManaPool from "./ManaPool";

export default function PlayerCard({ player, isActive, isPerspective, label }) {
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
        isActive && "border-[rgba(88,214,166,0.88)] shadow-[0_0_8px_rgba(88,214,166,0.25),0_0_0_1px_rgba(88,214,166,0.4)_inset]",
        isPerspective && "border-[rgba(100,169,255,0.88)] shadow-[0_0_0_1px_rgba(100,169,255,0.35)_inset]",
        !isActive && !isPerspective && "border-game-line-2"
      )}
      data-player-id={player.id}
    >
      <div className="flex items-center gap-2 min-w-0">
        <h2 className="text-[15px] font-bold m-0 truncate">{player.name}</h2>
        <ManaPool pool={player.mana_pool} />
      </div>

      <div className="flex flex-wrap gap-1 text-[11px] text-[#a8bfdd]">
        <span className="border border-[#223448] bg-[#0b121b] px-1.5 rounded-sm" title="Library">
          Lib <span className="font-bold text-[#d6e6fb]">{player.library_size}</span>
        </span>
        <span className="border border-[#223448] bg-[#0b121b] px-1.5 rounded-sm" title="Hand">
          Hand <span className="font-bold text-[#d6e6fb]">{player.hand_size}</span>
        </span>
        <span className="border border-[#223448] bg-[#0b121b] px-1.5 rounded-sm" title="Graveyard">
          GY <span className="font-bold text-[#d6e6fb]">{player.graveyard_size}</span>
        </span>
        <span className="border border-[#223448] bg-[#0b121b] px-1.5 rounded-sm" title="Exile">
          Exl <span className="font-bold text-[#d6e6fb]">{exileCards.length}</span>
        </span>
        <span className="border border-[#223448] bg-[#0b121b] px-1.5 rounded-sm" title="Battlefield">
          BF <span className="font-bold text-[#d6e6fb]">{battlefieldCount}</span>
        </span>
      </div>

    </section>
  );
}
