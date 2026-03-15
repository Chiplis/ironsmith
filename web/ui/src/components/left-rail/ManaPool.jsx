import { MANA_SYMBOLS } from "@/lib/constants";
import { ManaSymbol } from "@/lib/mana-symbols";

export default function ManaPool({ pool }) {
  if (!pool || typeof pool !== "object") return null;

  const chips = MANA_SYMBOLS.map(({ key, symbol, label }) => {
    const amount = Number(pool[key]);
    const safeAmount = Number.isFinite(amount) && amount > 0 ? Math.floor(amount) : 0;
    if (safeAmount <= 0) return null;
    return (
      <span
        key={key}
        className="inline-flex items-center gap-0.5 bg-[#0b121b] rounded-full px-1 py-px"
      >
        <span aria-label={`${safeAmount} ${label} mana in pool`} className="inline-flex items-center">
          <ManaSymbol sym={symbol} size={14} />
        </span>
        <span className="min-w-[7px] text-center text-[11px] leading-none font-bold text-[#d6e6fb]">
          {safeAmount}
        </span>
      </span>
    );
  }).filter(Boolean);

  if (!chips.length) return null;

  return <div className="flex flex-wrap items-center gap-1 ml-0.5">{chips}</div>;
}
