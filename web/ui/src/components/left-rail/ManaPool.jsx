import { MANA_SYMBOLS } from "@/lib/constants";

export default function ManaPool({ pool }) {
  if (!pool || typeof pool !== "object") return null;

  const chips = MANA_SYMBOLS.map(({ key, symbol, label, svg }) => {
    const amount = Number(pool[key]);
    const safeAmount = Number.isFinite(amount) && amount > 0 ? Math.floor(amount) : 0;
    if (safeAmount <= 0) return null;
    return (
      <span
        key={key}
        className="inline-flex items-center gap-0.5 bg-[#0b121b] rounded-full px-1 py-px"
        title={`${safeAmount} ${label} mana in pool`}
      >
        <img
          className="w-3.5 h-3.5 block shrink-0"
          src={svg}
          alt={`{${symbol}}`}
          loading="lazy"
          referrerPolicy="no-referrer"
        />
        <span className="min-w-[7px] text-center text-[11px] leading-none font-bold text-[#d6e6fb]">
          {safeAmount}
        </span>
      </span>
    );
  }).filter(Boolean);

  if (!chips.length) return null;

  return <div className="flex flex-wrap items-center gap-1 ml-0.5">{chips}</div>;
}
