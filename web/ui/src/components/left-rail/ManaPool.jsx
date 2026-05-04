import { MANA_SYMBOLS } from "@/lib/constants";
import { ManaSymbol } from "@/lib/mana-symbols";
import { cn } from "@/lib/utils";

export default function ManaPool({
  pool,
  alwaysVisible = false,
  compact = false,
  className = "",
}) {
  const safePool = pool && typeof pool === "object" ? pool : {};
  if (!alwaysVisible && safePool !== pool) return null;

  const chips = MANA_SYMBOLS.map(({ key, symbol, label }) => {
    const amount = Number(safePool[key]);
    const safeAmount = Number.isFinite(amount) && amount > 0 ? Math.floor(amount) : 0;
    if (!alwaysVisible && safeAmount <= 0) return null;
    return (
      <span
        key={key}
        className={cn(
          "mana-pool-chip inline-flex items-center gap-0.5 bg-background/70 rounded-full px-1 py-px",
          safeAmount <= 0 && "mana-pool-chip--empty",
          compact && "mana-pool-chip--compact"
        )}
      >
        <span aria-label={`${safeAmount} ${label} mana in pool`} className="inline-flex items-center">
          <ManaSymbol sym={symbol} size={compact ? 12 : 14} />
        </span>
        <span className="min-w-[7px] text-center text-[11px] leading-none font-bold text-foreground">
          {safeAmount}
        </span>
      </span>
    );
  }).filter(Boolean);

  if (!chips.length) return null;

  return (
    <div
      className={cn(
        "mana-pool-inline flex flex-wrap items-center gap-1 ml-0.5",
        alwaysVisible && "mana-pool-inline--persistent",
        compact && "mana-pool-inline--compact",
        className
      )}
      aria-label="Mana pool by type"
      title="Mana pool by type"
    >
      {chips}
    </div>
  );
}
