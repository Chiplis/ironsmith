import { scryfallImageUrl } from "@/lib/scryfall";

export default function StackCard({ entry, onClick }) {
  const name = entry.name || `Object#${entry.id}`;
  const artUrl = scryfallImageUrl(name, "art_crop");
  const scryfallUrl = scryfallImageUrl(name);

  return (
    <div
      className="game-card w-full min-w-0 min-h-[80px] text-[11px] border-[#80a8d7] bg-gradient-to-b from-[#132237] to-[#0d1726] cursor-pointer"
      data-object-id={entry.id}
      data-card-name={name}
      onClick={() => onClick?.(entry.id)}
    >
      {artUrl && (
        <div className="absolute inset-0 flex flex-col overflow-hidden z-0 pointer-events-none">
          <img
            className="w-full block object-cover opacity-72 saturate-[1.05] contrast-[1.04] flex-[0_0_40%] object-[center_top]"
            style={{
              maskImage: "linear-gradient(to bottom, black 40%, transparent 100%)",
              WebkitMaskImage: "linear-gradient(to bottom, black 40%, transparent 100%)",
            }}
            src={artUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
          />
          <img
            className="w-full block object-cover opacity-72 saturate-[1.05] contrast-[1.04] flex-[0_0_80%] -mt-[20%] object-[center_bottom]"
            style={{
              maskImage: "linear-gradient(to bottom, transparent 0%, black 18%)",
              WebkitMaskImage: "linear-gradient(to bottom, transparent 0%, black 18%)",
            }}
            src={artUrl}
            alt=""
            loading="lazy"
            referrerPolicy="no-referrer"
          />
        </div>
      )}

      {scryfallUrl && (
        <a
          className="absolute top-1 right-1 border border-[#4e6f93] bg-[#0a1118] text-[#9ec3ea] no-underline uppercase text-[9px] tracking-wide px-1 py-px rounded-sm leading-tight z-2 opacity-0 hover:opacity-100 transition-opacity"
          href={scryfallUrl}
          target="_blank"
          rel="noopener noreferrer"
          onClick={(e) => e.stopPropagation()}
        >
          img
        </a>
      )}

      <span className="relative z-2 leading-[1.12] text-shadow-[0_1px_1px_rgba(0,0,0,0.85)]">{name}</span>

      {entry.ability_kind ? (
        <span className="relative z-2 block text-[9px] italic text-[#c0a060] px-1 pt-0.5 leading-tight">
          {entry.ability_kind} ability
        </span>
      ) : entry.effect_text ? (
        <span className="relative z-2 block text-[9px] text-[#8ab4e0] px-1 pt-0.5 leading-tight overflow-hidden text-ellipsis max-h-[3.6em]">
          {entry.effect_text}
        </span>
      ) : null}
    </div>
  );
}
