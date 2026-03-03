import { cn } from "@/lib/utils";
import { scryfallImageUrl } from "@/lib/scryfall";

export default function GameCard({
  card,
  compact = false,
  isPlayable = false,
  isInspected = false,
  variant = "battlefield",
  onClick,
  style,
}) {
  const name = card.name || "";
  const artVersion = "art_crop";
  const artUrl = scryfallImageUrl(name, artVersion);
  const scryfallUrl = scryfallImageUrl(name);
  const count = Number(card.count);
  const groupSize = Number.isFinite(count) && count > 1 ? count : 1;

  return (
    <div
      className={cn(
        "game-card p-1.5 grid content-start",
        compact && "w-[96px] min-w-[96px] min-h-[134px] p-1 text-[14px]",
        !compact && variant === "hand" && "w-[124px] min-w-[124px] min-h-[100px]",
        !compact && variant !== "hand" && "w-[124px] min-w-[124px] min-h-[172px]",
        card.tapped && "tapped",
        isPlayable && "playable",
        isInspected && "inspected",
      )}
      data-object-id={card.id}
      data-card-name={name}
      title={groupSize > 1 ? `${name} (${groupSize} grouped permanents)` : name}
      onClick={onClick}
      style={style}
    >
      {artUrl && (
        <img
          className="absolute inset-0 w-full h-full object-cover opacity-72 z-0 pointer-events-none saturate-[1.05] contrast-[1.04]"
          src={artUrl}
          alt=""
          loading="lazy"
          referrerPolicy="no-referrer"
        />
      )}

      {/* Label pinned to top for battlefield cards */}
      <span className="absolute top-0 left-0 right-0 mt-0 bg-[rgba(16,24,35,0.85)] px-1.5 py-0.5 z-2 whitespace-nowrap overflow-hidden text-ellipsis text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]">
        {groupSize > 1 && (
          <span className="inline mr-1 border border-[rgba(245,198,112,0.9)] bg-[rgba(16,24,35,0.92)] text-[#f5d08b] text-[13px] font-bold leading-none px-1 py-0.5 rounded-sm tracking-wide">
            x{groupSize}
          </span>
        )}
        {name}
      </span>

      {/* P/T badge */}
      {card.power_toughness && (
        <span className="absolute bottom-1 right-1 border border-[rgba(245,198,112,0.9)] bg-[rgba(16,24,35,0.92)] text-[#f5d08b] text-[13px] font-bold leading-none px-1 py-0.5 rounded-sm z-2 tracking-wide">
          {card.power_toughness}
        </span>
      )}

      {/* Scryfall link */}
      {scryfallUrl && (
        <a
          className="absolute top-1 right-1 border border-[#4e6f93] bg-[#0a1118] text-[#9ec3ea] no-underline uppercase text-[12px] tracking-wide px-1 py-px rounded-sm leading-tight z-2 opacity-0 hover:opacity-100 group-hover:opacity-100 transition-opacity"
          href={scryfallUrl}
          target="_blank"
          rel="noopener noreferrer"
          draggable={false}
          title="Open Scryfall image"
          onClick={(e) => e.stopPropagation()}
        >
          img
        </a>
      )}
    </div>
  );
}
